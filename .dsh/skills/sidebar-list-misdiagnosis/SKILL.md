---
name: sidebar-list-misdiagnosis
description: >
  侧边栏会话列表「展开其余 N 个会话」点不动的误判排查（其实是滚动没到位，不是被盖住）
whenToUse: >
  怀疑侧边栏按钮被遮挡/点不动、要证明是不是插件问题时
---

# 侧边栏列表「点不动」的误判排查

## 侧边栏「展开其余 N 个会话」点不动？先别急着当 bug

> 2026-09-10 误判记录。

## 真相

会话列表容器 `[class*="listArea"]` / `[class*="list"]` 是**可滚动**的（`overflow-y: auto`，实测 client 340 / scroll 940）。列表最后那项「展开其余 N 个会话」经常在可视区之外，`getBoundingClientRect()` 仍会给出**可视区外**的坐标，往那个坐标派发鼠标事件自然打到底部用量面板（`cm-foot`）上——看起来像「被盖住 / 点不动」，其实只是**没滚动到位**。

## 复核手段

1. `document.elementFromPoint(btn 中心)` 若返回别的元素，先比较 `btn.getBoundingClientRect().top` 与滚动容器 `getBoundingClientRect().bottom`，确认是不是「屏幕外」；
2. 对照实验：用 `--profile minimal` 起同版本实例（没有 cost-meter 底栏、列表更短），同一个按钮若可点，即证明与插件无关。
