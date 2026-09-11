---
name: bubble-skin-vars
description: >
  气泡透明度/模糊与皮肤中心变量联动：--dsh-skin-bubble-alpha / --dsh-skin-bubble-blur。
---

# 气泡基材与皮肤中心变量联动

## 行内 CSS 变量来源
skin-center 的「背景」卡把值写在 `body` 的**行内 CSS 变量**上（`applyBubbleOpacity()` / `applyInputCardBlur()`，**与当前是哪张皮肤无关**，只要背景卡 `enabled` 就写；代码在 `@linxin666/dsh-client-ui-skin-center/lib/client.js` 的 `BUBBLE_ALPHA_VAR`）：
- `--dsh-skin-bubble-alpha` = 气泡不透明度 / 100（默认 50 → `0.5`）
- `--dsh-input-card-blur` = 输入卡片模糊（默认 10px）
- `--dsw-skin-scrim` = backgroundOpacity/100（壁纸遮罩）
- `--dsh-skin-bubble-blur` = **上游还没有**（本地补丁写的，见 `skin-center-knob-patch`）；提案在 [issue #1469](https://github.com/zhu1090093659/dsh-web/issues/1469)

## ⭐ 谁跟随滑块、谁不跟随（2026-09-11 用户明确指定）
- **跟随**：一切「文字背后的托底」—— markdown 块、disclosure 行、回合过程行、展开正文、表格托底。拖「气泡不透明度」= 它们一起变。
- **不跟随**：**窗口外框**（顶部导航栏、右侧栏面板）→ 固定 `rgba(242,245,250,.75)`。用户原话：「气泡不透明度不应该联动顶部导航和右侧边栏」。外框是框架，不是气泡。
- **亮色基色用左侧栏实测色 `rgb(242 245 250)`，不要纯白**（用户明确要求，纯白与周围面板不同色系）。

## 追加区改造（只动本地追加区，上游 262 行一字不动）
- **32 处**气泡底色改为 `rgb(R G B / calc(var(--dsh-skin-bubble-alpha, .5) * K))`
- K 取「**默认 50% 时正好复现原观感**」：`.5 → ×1`、`.55 → ×1.1`、`.75 → ×1.5`、`.4 → ×.8`、`.45 → ×.9`（深色同系数；超 1 由浏览器夹紧）
- **37 处**模糊改为 `blur(var(--dsh-skin-bubble-blur, 10px|12px)) saturate(1.3)`

## 实测结论（无头 Edge）
- 改 `--dsh-skin-bubble-alpha`：`0.8` → 胶囊 `.88`；`0.2` → `.22`；`0` → 全透明
- 同一测量里导航栏/右侧栏在 `0.8 / 0.2 / 0` 下**始终 `.75`**（固定密度，验证了「不跟随」）

## ⚠️ 坑
- **这个值可能是 0**。实测用户持久化配置 `~/.dsh/skin-center-active.json` 的 `background.bubbleOpacity` 就是 **0** → 联动一上线所有气泡立刻全透明，看起来像"皮肤坏了"。处置：把 `bubbleOpacity` 设回 50，或让用户在「设置 → **皮肤** → 背景」拖滑块（设置里的分区名叫「皮肤」，不是"皮肤中心"）。
- 页面以**加载时**读到的值为准：改完 JSON 必须**重新加载**页面，否则页面后续任何写入会把内存里的旧值（0）写回去。
- 官方推荐姿势可参考 whale-mom：`--dsw-specific-bubble: rgb(246 250 254 / var(--dsh-skin-bubble-alpha))` —— 我们只是把它用到 L3。
