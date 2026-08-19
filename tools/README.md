# tools 工具目录

本目录存放产测/硬件工具,按 **工具名 → 架构 → 操作系统 → 具体工具** 分层。
**具体工具二进制不上传 git**(体积大/各产线定制),由使用者自行放置。

## 目录结构

```
tools/
  <工具名>/
    <架构>/          # x86_64 / aarch64 ...
      <操作系统>/     # linux / windows ...
        具体工具       # 可执行文件 + 说明
```

## 当前预留工具

| 工具 | 路径 | 用途 | 来源 |
|---|---|---|---|
| ByoDmi | `tools/ByoDmi/x86_64/linux/ByoDmi` | DMI/SMBIOS 烧录与读取(SN/UUID) | TP100 产测工具包 |

## 放置说明

把工具按上述层级放入即可,程序启动时按 `tools/<工具名>/<架构>/<操作系统>/<工具>` 自动探测。

## 关联的 SN 读取方案

程序读取 SN 依次尝试:

1. `tools/ByoDmi/x86_64/linux/ByoDmi -smbiosinfo`(若放置,需 root)
2. `/sys/class/dmi/id/product_serial`(普通用户可读)
3. `/sys/class/dmi/id/board_serial`
4. `dmidecode -s system-serial-number`(需 root)
5. `/etc/machine-id`(兜底)

> Windows 用 `Win32_BIOS.SerialNumber`。
