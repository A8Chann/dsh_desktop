---
name: skin-center-knob-patch
description: >
  给皮肤中心加「气泡模糊程度」滑块、改 client.js 门面对象的补丁做法。
---

# 「气泡模糊程度」滑块本地补丁

## 背景
皮肤中心原本只有 5 个背景旋钮（背景遮挡 / 空对话背景模糊 / 有对话背景模糊 / **输入卡片模糊** / 气泡不透明度），**没有气泡模糊**。蓝幻想的 37 处 `blur(...)` 已改成读 `var(--dsh-skin-bubble-blur, 10px|12px)`，所以**只要有人写这个变量，滑块立刻生效**。

## 补丁脚本
- 路径：`scripts/skin-center-bubble-blur.mjs`
- 命令：`node scripts/skin-center-bubble-blur.mjs` 应用；`--check` 看状态；`--revert` 从同目录 `.orig-bubble-blur` 备份还原
- **共改 3 个文件**：
  - 客户端 = **`@linxin666/dsh-web-all/lib/client.js`**（真正下发的）+ `@linxin666/dsh-client-ui-skin-center/lib/client.js`（独立安装时用）
  - 宿主 = `@linxin666/dsh-client-ui-skin-center/lib/index.js`（defaults / RANGES / zod schema → 持久化）

## ⚠️ 怎么判定哪份代码在下发
拉首页，把里面 64 个 `/plugins/??…client.js` chunk 逐个抓下来找 `skin-center-bubble-opacity`——命中的是 **聚合包** `@linxin666/dsh-web-all/client.js`（2.5 MB，内联了 skin-center 的客户端代码）；宿主半边才是独立包（聚合包 `lib/shells/shell.js` 只有 `export { apply, inject }`，真插件由 `cordis.patch.yml` 的 `config.plugin: '@linxin666/dsh-client-ui-skin-center'` 指定）。**所以两个 client.js 都要打，只打独立包等于没打。**

## 🔥 最大的坑：组件拿到的是「门面对象」
- 组件拿到的 `background` 是逐条列方法（`bubbleOpacity: () => background.bubbleOpacity(), setBubbleOpacity: (v) => …`）的**门面对象**，**不是控制器实例**。
- 只给类加 getter/setter 而漏了门面 → `useSyncExternalStore(subscribe, undefined)` → **整个「设置 → 皮肤」面板崩掉**：控制台 `TypeError: n is not a function` + `slot entry crashed in 'settings.section'`。补门面两行后恢复。
- → 通用教训：给这类 controller 加字段，**类 + 门面必须同时改**；改完一定要看控制台有没有 `slot entry crashed`。

## 实证（无头 Edge）
F5 → 设置 → **皮肤**（这个分区在设置里叫「皮肤」，不是"皮肤中心"）→ 背景卡：
- `#skin-center-bubble-blur` 存在（`input[type=range]`，0–20，默认 10，aria「10px」）
- 拖到 18 → body `--dsh-skin-bubble-blur: 18px` → 胶囊 `blur(18px) saturate(1.3)`
- 把不透明度拖到 70% → `--dsh-skin-bubble-alpha: 0.7`、胶囊底 `rgba(242,245,250,0.77)`（= 0.7 × 1.1，联动系数正确）
- 截图 `.shot/blur-knob.png`

## ⚠️ 注意事项
- **持久化要重启后端**：宿主半边是进程内的模块，改完必须重启后端才会把 `bubbleBlur` 写进 `~/.dsh/skin-center-active.json`（实测未重启时该键被宿主丢弃 → 回落到默认 10）。客户端半边只需重新加载页面。
- **插件升级会覆盖这 3 个文件**（`dsh-web-all` 或 `skin-center` 任一升级）→ 重跑脚本即可（幂等；备份 `.orig-bubble-blur` 在各自目录）。
