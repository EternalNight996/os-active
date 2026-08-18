# os-active Git 双远程配置(GitHub + Gitee 双源)
# 用法: powershell -ExecutionPolicy Bypass -File scripts/setup-remotes.ps1
# 效果:
#   origin  = Gitee(既有主远程,git push origin master 照旧)
#   github  = GitHub 镜像/发布远程(CI 发布产物 + tag 推送)
# 之后双源推送:
#   git push origin master && git push github master
#   git push origin v0.1.0 && git push github v0.1.0
# 一键双源: git push origin master github master(git 2.x 支持多远程)
$ErrorActionPreference = 'Stop'

$gitee = 'git@gitee.com:eternalnight996/os-active.git'
$githubHttps = 'https://github.com/EternalNight996/os-active.git'
$githubSsh = 'git@github.com:EternalNight996/os-active.git'

Write-Host '=== os-active Git 双远程检查 ==='

$remotes = git remote
if (-not $remotes) { Write-Host '[缺] 无任何远程,先添加 Gitee 主远程'; git remote add origin $gitee }
elseif ($remotes -notcontains 'origin') { Write-Host '[缺] 缺少 origin(Gitee)'; git remote add origin $gitee }
else { Write-Host "[OK]  origin = $(git remote get-url origin)" }

if ($remotes -notcontains 'github') {
  Write-Host '[缺] 缺少 github 远程,自动添加(HTTPS)'
  git remote add github $githubHttps
  Write-Host "     如已配置 SSH key,可切换到 SSH: git remote set-url github $githubSsh"
} else {
  Write-Host "[OK]  github = $(git remote get-url github)"
}

Write-Host ''
Write-Host '=== 当前远程 ==='
git remote -v
Write-Host ''
Write-Host '=== 常用命令 ==='
Write-Host '  git push origin master github master   # 双源推代码'
Write-Host '  git push origin v0.1.0 github v0.1.0   # 双源推 tag(触发 CI 发布)'
Write-Host '  git fetch origin github                # 拉双源'
