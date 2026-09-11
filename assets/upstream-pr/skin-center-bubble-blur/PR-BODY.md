## 摘要 / Summary

皮肤中心「背景」卡现在有 5 个旋钮（背景遮挡 / 空对话背景模糊 / 有对话背景模糊 / 输入卡片模糊 / 气泡不透明度），**没有气泡模糊**。而「气泡更透」与「气泡更糊」是两个独立诉求：插画 / 贴纸壁纸下把不透明度调到 0 时，不该连磨砂一起消失；想保留底色、单独调磨砂时也没有入口。

本 PR 对称补齐 [#783](https://github.com/zhu1090093659/dsh-web/issues/783)（输入卡模糊滑杆）建立的产品面，在「气泡不透明度」旁边新增第 6 个旋钮。

The Background card ships five knobs today and no bubble blur. Bubble transparency and bubble blur are independent wishes: with an illustrated wallpaper, pushing opacity to 0 must not take the frosting with it. This adds the sixth knob next to “Bubble opacity”, mirroring the product surface #783 established for the input card.

## 契约 / Contract

| | |
| --- | --- |
| 设置字段 / field | `bubbleBlur`（命名空间 `skin-background`，与 `bubbleOpacity` 同级） |
| 范围 / 步进 | `0–20` px，step `1` |
| 默认 / default | **10** px（= 既有「输入卡片模糊」默认档：采用该变量的皮肤观感不变） |
| body 变量 / CSS var | `--dsh-skin-bubble-blur`（如 `10px`），与 `--dsh-skin-bubble-alpha` 并列 |
| 可选性 / optional | 皮肤不读该变量 = 零变化；宿主侧为纯新增可选字段，旧配置无需迁移 |
| 主开关 / master switch | 皮肤中心关闭时移除该变量（与既有 alpha 一致） |

皮肤侧用法（没有本 PR 时靠 fallback 也成立）/ Skin-side usage (works via fallback even before this lands):

```css
backdrop-filter: blur(var(--dsh-skin-bubble-blur, 10px)) saturate(1.3);
```

## 改动 / Changes

- `src/core/background.ts` — `SkinBackgroundConfig.bubbleBlur?`、默认值、`RANGES`（自动覆盖 normalize / sanitize / resolve / FIELDS）
- `src/index.ts` — zod schema
- `src/client/background.ts` — `BUBBLE_BLUR_FIELD` / `BUBBLE_BLUR_VAR` / `DEFAULT_BUBBLE_BLUR`；`SkinBackgroundHandle` 的两个方法；控制器字段、构造与 `init()` 应用、`snapshot()`、getter、setter、`dispose()` 清理、`assign()`、`applyBubbleBlur()`
- `src/client/index.ts` — injected handle 门面（**漏这里 typecheck 会以 TS2739 失败，组件运行时也会崩**）
- `src/client/SkinCenter.tsx` — 两个 hook + 第 6 个 `SliderControl`（`id="skin-center-bubble-blur"`，紧邻「气泡不透明度」）
- `src/client/locales.ts` — 键联合类型 + `en` / `zh` 词条
- 测试 / Tests — `background.spec.ts`（新增 “applies, persists, and cleans up message bubble blur”，并补默认值与 snapshot 断言）、`background-migration.spec.ts`、`background-scope.spec.ts`、`skin-center-custom-theme.spec.tsx`（handle mock + 滑块交互用例）、`routes-v2.spec.ts`（区间夹紧覆盖）

## 验证 / Verification

- `pnpm --filter @linxin666/dsh-client-ui-skin-center test` → **35 files / 618 tests passed**
- `pnpm --filter @linxin666/dsh-client-ui-skin-center typecheck` → clean
- `tsdown` 构建通过（`pnpm install` 的 `prepare` 步骤）
- ⚠️ 未跑全仓 `make check`（本次只安装了该包及其依赖）/ Full-repo `make check` not run locally

## 与皮肤侧的关系 / Relation to skins

该变量按**可选契约**设计：皮肤照上面那行写即可接住（whale-mom 已在用同模式的 `--dsh-skin-bubble-alpha`）。蓝幻想皮肤侧消费它的 L3 可读性补丁会另开一个 PR。
