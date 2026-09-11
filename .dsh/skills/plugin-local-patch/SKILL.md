---
name: plugin-local-patch
description: >
  修改第三方插件产物、做本地回退打补丁（.orig 备份、node --check、升级后重打）。
---

# 第三方插件本地回退 / 打补丁

## 子 agent 自动打开位置（dsh-better-sidebar 0.19 起写死为底部面板）

### 现象
调用子 agent 时弹**底部面板**而不是右侧栏。

### 根因
`~/.dsh/profiles/web/node_modules/dsh-better-sidebar/lib/client.js` 里自动打开逻辑改成：

```js
openTab({ type: "subagent", title: t("subagent"), target: "bottom" })
```

共 **3 处**（两个 effect + subagentJump）。

**0.18.0 时是无 target** —— `openTab` 内部 `if (surface !== void 0 && seed.target !== "bottom")` 才走右侧栏 surface 分支，并带 `revealIfOpened` 自动展开；旧版还会先 `store.reduce(... togglePanel ...)` 打开面板。

今天随 `@linxin666/dsh-web-all@0.3.20` 把 better-sidebar 从 0.18.0 升到 **0.19.0-alpha.1** 才变了行为。

### 回退
把那 3 处的 `, target: "bottom"` 删掉（**保留** terminal / jobs 的 `target: "bottom"` 不动），`node --check` 验证语法；备份留在 `lib/client.js.orig-subagent-bottom`。

**⚠️ 插件升级会被覆盖，需重打。**

### 不想打补丁的替代方案
better-sidebar 设置里关掉「自动打开子 agent」（`autoOpenSubagent`，默认 true），改成手动点右侧栏的 subagent 标签页。

## 通用原则

- 改插件产物前先备份为 `*.orig-<说明>`。
- 改完必须跑语法检查（JS 用 `node --check`）。
- 在 `node_modules` 里用 `Select-String -Pattern "<哈希前缀>|<slot名>"` 确认某段 UI 到底属于哪个包。
