### 提交前查重
- [x] 我已搜索过 open/closed 的 Issue，确认本 Issue 没有重复。（搜索 `bubble` / `模糊`，相关既有条目：#486、#602、#528 是气泡**不透明度**，[#783](https://github.com/zhu1090093659/dsh-web/issues/783) 是**输入卡片**模糊，均未覆盖气泡模糊）

### 涉及插件
皮肤 / 皮肤中心 (dsh-skins)

### Issue 类型
功能请求

### 摘要
皮肤中心「背景」卡已有 5 个旋钮（背景遮挡 / 空对话背景模糊 / 有对话背景模糊 / 输入卡片模糊 / 气泡不透明度），**没有气泡模糊**。请求新增第 6 个——「气泡模糊程度」（`0–20` px，默认 **10** px），落在 `body` 的 `--dsh-skin-bubble-blur` 上，与既有 `--dsh-skin-bubble-alpha` 并列。

### 预期结果
1. 「背景」卡在「气泡不透明度」下方多一行「气泡模糊程度」滑杆（0–20，step 1，默认 10）；
2. 该值以 `--dsh-skin-bubble-blur: 10px` 的形式写在 `document.body` 上（与 `--dsh-skin-bubble-alpha: 0.5` 同一机制，皮肤中心关闭时一并移除）；
3. 皮肤按可选契约消费，例如：
   ```css
   backdrop-filter: blur(var(--dsh-skin-bubble-blur, 10px)) saturate(1.3);
   ```
   不读该变量的皮肤**零变化**；宿主侧为纯新增可选字段，旧 `skin-center-active.json` 无需迁移。

### 详情 / 复现步骤
**使用场景与约束**
- 「更透」与「更糊」是两个独立诉求。插画 / 贴纸壁纸下把「气泡不透明度」调到 0 时，气泡背后的磨砂会一起消失；反之想保留底色、只调磨砂强度时也没有入口。（本地实测：不透明度 `0.776 → 0.08` 时视觉上就是"气泡变透明了"，而加回模糊后是"变磨砂"，观感完全不同。）
- 默认值取 **10 px**，与既有「输入卡片模糊」的默认档一致 —— 这样采用该变量的皮肤在不改设置时观感不变。
- 这是把 [#783](https://github.com/zhu1090093659/dsh-web/issues/783) 已建立的产品面（给输入卡加模糊滑杆）**对称补到气泡这一层**：既然 `--dsh-skin-bubble-alpha` 已经是公开契约，再补一个模糊通道是自然的延伸。

**实现已备好（如接受，可立即开 PR）**
- 分支（我方 fork，仅备用，未开 PR）：`A8Chann/dsh-web` @ `feat/skin-center-bubble-blur`
- 补丁规模：**11 个文件、+109 / −3**（6 个源文件 + 5 个测试文件）
  - `src/core/background.ts`（接口 + 默认值 + `RANGES`）、`src/index.ts`（zod）、`src/client/background.ts`（变量常量 + `SkinBackgroundHandle` + 控制器）、`src/client/index.ts`（injected handle 门面）、`src/client/SkinCenter.tsx`（hook + 第 6 个 `SliderControl`）、`src/client/locales.ts`（键 + en/zh 词条）
  - 测试：`background.spec.ts` 新增 “applies, persists, and cleans up message bubble blur”，并同步更新 `background-migration` / `background-scope` / `skin-center-custom-theme`（handle mock + 滑块交互）/ `routes-v2`（区间夹紧）
- 本地验证：
  - `pnpm --filter @linxin666/dsh-client-ui-skin-center test` → **35 files / 618 tests passed**
  - `pnpm --filter @linxin666/dsh-client-ui-skin-center typecheck` → clean
  - `tsdown` 构建通过
  - （未跑全仓 `make check`：本地只安装了该包及其依赖）
- 截图（本地实现效果，拖到 18px 时气泡底实时变糊）：
  `https://raw.githubusercontent.com/A8Chann/dsh-web/assets/skin-center-bubble-blur/assets-issue/blur-knob.png`

### 环境信息
- DSH 版本: 0.1.5-rc.1
- 浏览器: WebView2（DSH Desktop 2.7.0 内嵌，Chromium 152）
- 插件名称 / 版本: `@linxin666/dsh-web-all` 0.3.20（skin-center 0.3.20）
- 操作系统: Windows 11（profile = `web`）

### 补充信息
配套的皮肤侧改动（blue-fantasy L3 可读性补丁）会**另开 PR**（属于「皮肤 / 皮肤中心」类别），其中气泡相关的 37 处 `blur()` 已经在读 `var(--dsh-skin-bubble-blur, 10px)` —— 也就是说变量一旦存在就立刻生效，无需再动皮肤。

<details><summary>English summary</summary>

The Background card ships five knobs and no bubble blur. Bubble **transparency** (`--dsh-skin-bubble-alpha`) and bubble **blur** are independent wishes — with an illustrated wallpaper, pushing opacity to 0 should not take the frosting with it. This requests a sixth knob, `bubbleBlur` (`0–20` px, default **10** px), written to `--dsh-skin-bubble-blur` on `document.body`, symmetric to the input-card blur control added in #783. It is an optional variable: skins that do not read it are unaffected and no config migration is needed. A ready-to-review implementation (11 files, +109/−3, 618 package tests and typecheck green) lives on `A8Chann/dsh-web@feat/skin-center-bubble-blur`; happy to open the PR if you want it.
</details>
