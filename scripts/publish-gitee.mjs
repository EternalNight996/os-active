#!/usr/bin/env node
// 发布安装包到 Gitee 发行版(与 GitHub Release 双源互备,供国内用户走 Gitee)
//   1. 收集构建产物:*.zip(Windows)+ *.deb(Linux)
//   2. 确保 Gitee release 存在(tag 需已推送到 Gitee 仓库)
//   3. 上传全部安装包附件,输出真实下载 URL
//
// 前置:
//   GITEE_TOKEN = Gitee 私人令牌(gitee.com -> 设置 -> 安全设置 -> 私人令牌,勾选 projects/releases)
//   node scripts/publish-gitee.mjs --tag v0.1.0 --artifacts-dir artifacts --repo eternalnight996/os-active
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join, dirname, basename } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const args = process.argv.slice(2);
const flag = (name, dflt) => {
  // 同时支持 --name=value 与 --name value(CI 里用空格分隔)
  const i = args.indexOf(name);
  if (i !== -1 && args[i + 1] !== undefined) return args[i + 1];
  const hit = args.find((a) => a.startsWith(name + '='));
  return hit ? hit.slice(name.length + 1) : dflt;
};

const token = process.env.GITEE_TOKEN || process.env.GITEE_ACCESS_TOKEN;
if (!token) { console.error('未设置 GITEE_TOKEN(Gitee 私人令牌)'); process.exit(1); }

const version = (readFileSync(join(root, 'Cargo.toml'), 'utf8').match(/^version = "([^"]+)"/m) || [])[1];
if (!version) { console.error('无法从 Cargo.toml 读取版本'); process.exit(1); }
const tag = flag('--tag', 'v' + version);
const [owner, repo] = flag('--repo', 'eternalnight996/os-active').split('/');
const artifactsDir = flag('--artifacts-dir', join(root, 'dist'));
console.log(`Gitee 发布 ${tag} -> ${owner}/${repo}(版本 ${version})`);

const api = 'https://gitee.com/api/v5';
const headers = { 'User-Agent': 'publish-gitee.mjs' };
async function apiJson(path, opts = {}) {
  const url = opts.params ? api + path + '?' + new URLSearchParams(opts.params) : api + path;
  const res = await fetch(url, { method: opts.method || 'GET', headers, body: opts.body });
  const text = await res.text();
  let json = null;
  try { json = text ? JSON.parse(text) : null; } catch {}
  return { status: res.status, json, text };
}

// ---- 1. 收集产物(zip + deb,递归扫描 CI 下载的 artifacts 目录)----
const installerExts = ['.zip', '.deb'];
const installers = [];
function walk(dir) {
  if (!existsSync(dir)) return;
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    const st = existsSync(p) ? statSync(p) : null;
    if (st?.isDirectory()) walk(p);
    else if (installerExts.some((e) => name.endsWith(e))) installers.push(p);
  }
}
walk(artifactsDir);
if (!installers.length) { console.error('未找到安装包(*.zip / *.deb):', artifactsDir); process.exit(1); }
console.log('安装包:', installers.map((p) => basename(p)).join(', '));

// ---- 2. 确保 Gitee release 存在(Gitee 对不存在的 tag 可能返回 200+null,需兜底)----
let releaseId = null;
const existing = await apiJson('/repos/' + owner + '/' + repo + '/releases/tags/' + tag, { params: { access_token: token } });
if (existing.status === 200 && existing.json && existing.json.id) {
  releaseId = existing.json.id;
  console.log('Gitee release 已存在,id=', releaseId);
} else {
  const form = new URLSearchParams({
    access_token: token,
    tag_name: tag,
    name: tag,
    body: flag('--notes', 'os-active ' + tag + ' 跨平台发布(Windows zip + Linux deb)'),
    target_commitish: 'master',
  });
  const created = await apiJson('/repos/' + owner + '/' + repo + '/releases', { method: 'POST', body: form });
  if (created.status >= 200 && created.status < 300 && created.json && created.json.id) {
    releaseId = created.json.id;
    console.log('Gitee release 已创建,id=', releaseId);
  } else {
    // 兜底:从发行版列表按 tag 查找
    const list = await apiJson('/repos/' + owner + '/' + repo + '/releases', { params: { access_token: token, per_page: 100 } });
    const hit = (list.json || []).find((r) => r.tag_name === tag);
    if (hit && hit.id) {
      releaseId = hit.id;
      console.log('从列表找到已有 Gitee release,id=', releaseId);
    } else {
      console.error('创建/查找 Gitee release 失败:', created.status, created.text);
      process.exit(1);
    }
  }
}

// ---- 3. 上传附件(Gitee 附件上限 100MB,超限跳过)----
const GITEE_MAX_FILE = 100 * 1024 * 1024;
const uploadable = [];
for (const f of installers) {
  const size = statSync(f).size;
  if (size > GITEE_MAX_FILE) {
    console.warn('跳过(超过 Gitee 100MB 上限):', basename(f), Math.round(size / 1024 / 1024) + 'MB');
    continue;
  }
  uploadable.push(f);
}
if (!uploadable.length) { console.error('没有可上传的安装包'); process.exit(1); }
const downloadUrls = {}; // 文件名 -> browser_download_url
for (const f of uploadable) {
  const name = basename(f);
  const body = new FormData();
  body.append('access_token', token);
  body.append('file', new Blob([readFileSync(f)]), name);
  const up = await fetch(api + '/repos/' + owner + '/' + repo + '/releases/' + releaseId + '/attach_files', {
    method: 'POST',
    headers,
    body,
  });
  const text = await up.text();
  let json = null;
  try { json = JSON.parse(text); } catch {}
  if (up.status >= 200 && up.status < 300 && json?.browser_download_url) {
    downloadUrls[name] = json.browser_download_url;
    console.log('已上传', name, '->', json.browser_download_url);
  } else {
    console.error('上传失败', name, up.status, text);
    process.exit(1);
  }
}

console.log('Gitee 发布完成:', 'https://gitee.com/' + owner + '/' + repo + '/releases/tag/' + tag);
console.log('下载地址:');
for (const [name, url] of Object.entries(downloadUrls)) console.log('  ' + name + ' -> ' + url);
