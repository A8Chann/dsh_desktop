---
name: backend-management
description: >
  改后端进程管理时读：spawn/adopt/restart、重启死锁根因、退避重试、状态机。
---

# 后端管理（backend.rs）

- 单管理线程 `run_loop` 循环：`spawn_own`（拉起 `node dsh bin.js web --no-open`）/ `adopt_external`（端口被 dsh 占用则接管，轮询 1s）/ `fail`（退避重试 1s→30s 封顶）。
- **重启必须「先杀后 join」**：管理线程阻塞在子进程 stdout 的 `read_line` 上，子进程不退出则 `read_line` 永不返回，直接 `join()` 会**永久死锁**（重启失效的根因）。`restart()` = `kill_owned()` → `join()` → `start()` → 后台轮询到 running 后 `location.reload()`。
- 退避 sleep 按 **1 秒切片**并每片检查 `stop`，保证重启/退出能及时打断（最长 30s 的整段 sleep 会让 join 卡住）。
- 状态机：idle / starting / running / external / error / restarting / stopped，经 `backend-status` 事件 + `win.eval` 推送到注入标题栏。
- **启动命令必须是 `node <bin.js> --profile <name> --no-open --port <port>`**：`web` 只是 `--profile web` 的**别名**，不能与 `--profile <name>` 混用（`dsh --profile x web ...` 会被 Commander 拒绝）；自定义 profile 直接传 web 自身参数即可。`controls.rs::run_install_plugin` 使用 `dsh plugin --profile <name> ...` 的形式（子命令前无 parent `--profile`），同样按当前 settings.profile，不再硬编码 web。
