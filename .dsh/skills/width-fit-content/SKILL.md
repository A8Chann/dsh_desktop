---
name: width-fit-content
description: >
  元素宽度自适应、shrink-to-fit、内容被挤换行时的正确写法。
---

# 宽度自适应 / 收缩包裹（shrink-to-fit）规则

## 「本次产出」行（deliverables）

类名 `nArs4W_producedRow` 之前没有补丁覆盖 → 补玻璃底 + **自适应宽度**。

### ⚠️ 不能裸用 `width: fit-content` 单干

它的父级是 `display: contents` 的插槽宿主（宽度算出来是 0），`fit-content + max-width:100%` 会把里面的文件 chip 挤成 3 行（实测 434px / 3 行）。

### 正解：行内级 shrink-to-fit 盒

```css
.nArs4W_producedRow {
  display: inline-flex;
  align-self: flex-start;   /* 防父级 flex stretch */
  width: fit-content;
  max-width: 100%;
}
```

不依赖父宽、也不怕父级是 flex。

### 验证方法

用**同构合成 DOM**（block 737px 父 > display:contents 宿主 > 行）在真实样式表下验证：
- 背景宽 281px / 内容 243px / **1 行不换行** ✓

## 通用原则

- 父级是 `display: contents` 或 0 宽度时，`width: fit-content` 单独用**无效**，必须配合 `display: inline-flex / inline-block` 与 `align-self`。
- dock 子元素（统计行等）的通栏问题参见 skill `composer-dock`。
