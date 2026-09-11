---
name: logging-troubleshooting
description: >
  日志位置与排障锚点、状态字段 snake_case 命名坑。
---

# 日志与排障

- 日志：`%APPDATA%\DSH Desktop\logs\main.log`（UTF-8；PowerShell 控制台按 GBK 显示会乱码，读文件用 `-Encoding UTF8`）。
- 关键锚点：「==== 重启后端」「==== 终止自有后端进程树 pid=」「[http] action:」「==== DSH Desktop 退出」「[close] 页面不可用」。
- 排障先看日志时间线，能直接区分「action 没走到 / 回调没执行 / pid 没找到 / 杀进程失败」。
