#!/usr/bin/env node
/**
 * 给「皮肤中心」补一个「气泡模糊」滑块（本地补丁）
 *
 * 背景：皮肤中心的「背景」卡已有 5 个旋钮（壁纸不透明度 / 空态模糊 / 有内容模糊 /
 * 输入卡片模糊 / 气泡不透明度），但没有"气泡模糊"。本脚本在同一个位置补第 6 个：
 *   - 客户端：加 `--dsh-skin-bubble-blur`（px）写入 body，并加滑块 UI / i18n 词条
 *   - 宿主：把 `bubbleBlur`（0–20，默认 10）加进 defaults / RANGES / zod schema，
 *           这样它才会被规范化并持久化到 ~/.dsh/skin-center-active.json
 *
 * 蓝幻想皮肤的 patches.css 已经读这个变量（`blur(var(--dsh-skin-bubble-blur, 10px))`），
 * 所以补丁生效后滑块立刻控制所有气泡底的毛玻璃强度。
 *
 * 用法（在仓库根）：
 *   node scripts/skin-center-bubble-blur.mjs            # 应用（幂等）
 *   node scripts/skin-center-bubble-blur.mjs --check    # 只看状态
 *   node scripts/skin-center-bubble-blur.mjs --revert   # 还原
 *   node scripts/skin-center-bubble-blur.mjs --profile web
 *
 * ⚠️ 插件升级（dsh-web-all / skin-center）会覆盖这些文件 → 升级后重跑本脚本。
 * ⚠️ 宿主半边改动需要**重启后端**才生效（只是不持久化而已，滑块本身实时可用）。
 */
import { readFileSync, writeFileSync, existsSync, copyFileSync } from 'node:fs';
import { join } from 'node:path';
import { homedir } from 'node:os';

const argv = process.argv.slice(2);
const check = argv.includes('--check');
const revert = argv.includes('--revert');
const pi = argv.indexOf('--profile');
const profile = pi >= 0 ? argv[pi + 1] : 'web';
const home = process.env.DSH_HOME || join(homedir(), '.dsh');
const nm = join(home, 'profiles', profile, 'node_modules');

/** 「气泡模糊程度」随哪个上游版本正式发布（见 dsh-web PR #1516，merge ffedeae）。 */
const UPSTREAM_VERSION = [0, 3, 22];

/**
 * 上游是否已经自带这个功能。≥ 0.3.22 就必须**停止打补丁** ——
 * 否则等于把同一功能实现两遍，且下面的 --check 会误报。
 */
function upstreamHasFeature() {
  const pkg = join(nm, '@linxin666', 'dsh-client-ui-skin-center', 'package.json');
  if (!existsSync(pkg)) return false;
  try {
    const v = JSON.parse(readFileSync(pkg, 'utf8')).version;
    const cur = String(v).split('.').map(Number);
    for (let i = 0; i < 3; i++) {
      const a = cur[i] ?? 0;
      const b = UPSTREAM_VERSION[i];
      if (a !== b) return a > b;
    }
    return true;
  } catch {
    return false;
  }
}

if (upstreamHasFeature()) {
  console.log(`皮肤中心「气泡模糊」补丁 —— **已废弃，无需再打**。`);
  console.log(`  已安装的 @linxin666/dsh-client-ui-skin-center 版本 ≥ ${UPSTREAM_VERSION.join('.')}，`);
  console.log(`  该功能已随上游正式发布（dsh-web PR #1516，merge ffedeae），原版即带「气泡模糊程度」滑杆。`);
  console.log(`  本地产物热修已撤回；继续打补丁会把同一功能实现两遍。`);
  console.log(`  如需在旧版本上临时启用，请先降级到 < ${UPSTREAM_VERSION.join('.')} 再运行本脚本。`);
  process.exit(0);
}

const CLIENT_FILES = [
  join(nm, '@linxin666', 'dsh-web-all', 'lib', 'client.js'),              // 实际下发的（聚合包内联副本）
  join(nm, '@linxin666', 'dsh-client-ui-skin-center', 'lib', 'client.js') // 独立安装时用的那份
];
const HOST_FILES = [
  join(nm, '@linxin666', 'dsh-client-ui-skin-center', 'lib', 'index.js'),
  join(nm, '@linxin666', 'dsh-web-all', 'lib', 'index.js')
];

const IND = '([ \\t]*)';
const NL = '(\\r?\\n)';

// ── 客户端补丁 ────────────────────────────────────────────────────────────
const UI_ROW = `$1/* 本地补丁（skin-center-bubble-blur.mjs）：气泡毛玻璃强度 */ (0, react_jsx_runtime.jsxs)("div", {
$1\tclassName: skin_center_module_css_default.backgroundRow,
$1\tchildren: [
$1\t\t(0, react_jsx_runtime.jsxs)("div", {
$1\t\t\tclassName: skin_center_module_css_default.backgroundHead,
$1\t\t\tchildren: [(0, react_jsx_runtime.jsx)("span", {
$1\t\t\t\tclassName: skin_center_module_css_default.backgroundLabel,
$1\t\t\t\tchildren: t("bubbleBlur")
$1\t\t\t}), (0, react_jsx_runtime.jsxs)("span", {
$1\t\t\t\tclassName: skin_center_module_css_default.backgroundValue,
$1\t\t\t\t"aria-hidden": "true",
$1\t\t\t\tchildren: [shownBubbleBlur, "px"]
$1\t\t\t})]
$1\t\t}),
$1\t\t(0, react_jsx_runtime.jsx)(SliderControl, {
$1\t\t\tid: "skin-center-bubble-blur",
$1\t\t\tclassName: skin_center_module_css_default.backgroundRange,
$1\t\t\tmin: 0,
$1\t\t\tmax: 20,
$1\t\t\tstep: 1,
$1\t\t\tvalue: bubbleBlur,
$1\t\t\tariaValuetext: shownBubbleBlur + "px",
$1\t\t\tariaLabel: t("bubbleBlur"),
$1\t\t\tonChanging: setShownBubbleBlur,
$1\t\t\tonChange: (value) => {
$1\t\t\t\tbackground.setBubbleBlur(value);
$1\t\t\t}
$1\t\t}),
$1\t\t(0, react_jsx_runtime.jsx)("p", {
$1\t\t\tclassName: skin_center_module_css_default.backgroundHint,
$1\t\t\tchildren: t("bubbleBlurHint")
$1\t\t})
$1\t]
$1}),
`;

const CLIENT_PATCHES = [
  { name: 'defaults 镜像 +bubbleBlur', find: new RegExp(`^${IND}bubbleOpacity: 50$`, 'm'), repl: '$1bubbleOpacity: 50,\n$1bubbleBlur: 10', once: true },
  { name: 'RANGES 镜像 +bubbleBlur', find: new RegExp(`^${IND}bubbleOpacity: \\[0, 100\\]$`, 'm'), repl: '$1bubbleOpacity: [0, 100],\n$1bubbleBlur: [0, 20]', once: true },
  {
    name: '声明 BUBBLE_BLUR_VAR',
    find: new RegExp(`^${IND}const BUBBLE_ALPHA_VAR = "--dsh-skin-bubble-alpha";$`, 'm'),
    repl: '$1const BUBBLE_ALPHA_VAR = "--dsh-skin-bubble-alpha";\n$1/** 本地补丁：气泡毛玻璃强度（px），由「背景 → 气泡模糊」滑块写入。 */\n$1const BUBBLE_BLUR_VAR = "--dsh-skin-bubble-blur";',
    once: true
  },
  { name: 'defaults 引用行', find: new RegExp(`^${IND}SKIN_BACKGROUND_DEFAULTS\\.bubbleOpacity;$`, 'm'), repl: '$1SKIN_BACKGROUND_DEFAULTS.bubbleOpacity;\n$1SKIN_BACKGROUND_DEFAULTS.bubbleBlur;', once: true },
  { name: '字段初始化', find: new RegExp(`^${IND}bubbleOpacityValue = SKIN_BACKGROUND_DEFAULTS\\.bubbleOpacity;$`, 'm'), repl: '$1bubbleOpacityValue = SKIN_BACKGROUND_DEFAULTS.bubbleOpacity;\n$1bubbleBlurValue = SKIN_BACKGROUND_DEFAULTS.bubbleBlur;', once: true },
  { name: 'apply 调用点 ×3', find: new RegExp(`^${IND}this\\.applyBubbleOpacity\\(\\);${NL}${IND}this\\.syncBlur\\(\\);`, 'gm'), repl: '$1this.applyBubbleOpacity();$2$3this.applyBubbleBlur();$2$3this.syncBlur();' },
  { name: 'snapshot() +bubbleBlur', find: new RegExp(`^${IND}bubbleOpacity: this\\.bubbleOpacityValue$`, 'm'), repl: '$1bubbleOpacity: this.bubbleOpacityValue,\n$1bubbleBlur: this.bubbleBlurValue', once: true },
  { name: 'getter bubbleBlur()', find: new RegExp(`^${IND}bubbleOpacity = \\(\\) => this\\.bubbleOpacityValue;$`, 'm'), repl: '$1bubbleOpacity = () => this.bubbleOpacityValue;\n$1bubbleBlur = () => this.bubbleBlurValue;', once: true },
  {
    name: 'setter setBubbleBlur()',
    find: new RegExp(`^${IND}setBubbleOpacity\\(value\\) \\{${NL}([\\s\\S]*?)${NL}\\1\\}`, 'm'),
    repl: '$&' + `\n$1setBubbleBlur(value) {\n$1\tthis.bubbleBlurValue = this.clampBlur(value);\n$1\tthis.applyBubbleBlur();\n$1\tthis.publish();\n$1\tthis.persist(this.snapshot());\n$1}`,
    once: true
  },
  { name: 'dispose 清理', find: new RegExp(`^${IND}document\\.body\\.style\\.removeProperty\\(BUBBLE_ALPHA_VAR\\);$`, 'm'), repl: '$1document.body.style.removeProperty(BUBBLE_ALPHA_VAR);\n$1document.body.style.removeProperty(BUBBLE_BLUR_VAR);', once: true },
  { name: 'assign() +bubbleBlur', find: new RegExp(`^${IND}this\\.bubbleOpacityValue = resolved\\.bubbleOpacity;$`, 'm'), repl: '$1this.bubbleOpacityValue = resolved.bubbleOpacity;\n$1this.bubbleBlurValue = resolved.bubbleBlur;', once: true },
  {
    name: 'applyBubbleBlur() 方法',
    find: new RegExp(`^${IND}applyBubbleOpacity\\(\\) \\{${NL}([\\s\\S]*?)${NL}\\1\\}`, 'm'),
    repl: '$&' + `\n$1applyBubbleBlur() {\n$1\tif (!this.enabledValue) {\n$1\t\tdocument.body.style.removeProperty(BUBBLE_BLUR_VAR);\n$1\t\treturn;\n$1\t}\n$1\tdocument.body.style.setProperty(BUBBLE_BLUR_VAR, this.bubbleBlurValue + "px");\n$1}`,
    once: true
  },
  { name: 'UI hook: bubbleBlur store', find: new RegExp(`^${IND}const bubbleOpacity = \\(0, react\\.useSyncExternalStore\\)\\(background\\.subscribe, background\\.bubbleOpacity\\);$`, 'm'), repl: '$&' + '\n$1const bubbleBlur = (0, react.useSyncExternalStore)(background.subscribe, background.bubbleBlur);', once: true },
  { name: 'UI hook: useLiveValue', find: new RegExp(`^${IND}const \\[shownBubbleOpacity, setShownBubbleOpacity\\] = useLiveValue\\(bubbleOpacity\\);$`, 'm'), repl: '$&' + '\n$1const [shownBubbleBlur, setShownBubbleBlur] = useLiveValue(bubbleBlur);', once: true },
  // ⚠️ 关键：组件拿到的 `background` 是**门面对象**（逐条列方法），不是控制器实例。
  // 只给类加 getter/setter 而漏了门面 → useSyncExternalStore(subscribe, undefined) → 面板整段崩溃
  // （实测 `TypeError: n is not a function` + `slot entry crashed in 'settings.section'`）。
  { name: '门面: bubbleBlur', find: new RegExp(`^${IND}bubbleOpacity: \\(\\) => background\\.bubbleOpacity\\(\\),$`, 'm'), repl: '$&' + '\n$1bubbleBlur: () => background.bubbleBlur(),', once: true },
  { name: '门面: setBubbleBlur', find: new RegExp(`^${IND}setBubbleOpacity: \\(value\\) => background\\.setBubbleOpacity\\(value\\),$`, 'm'), repl: '$&' + '\n$1setBubbleBlur: (value) => background.setBubbleBlur(value),', once: true },
  { name: 'UI 滑块（插在 WallpaperPanel 前）', find: new RegExp(`^${IND}/\\* @__PURE__ \\*/ \\(0, react_jsx_runtime\\.jsx\\)\\(WallpaperPanel, \\{`, 'm'), repl: UI_ROW + '$&', once: true },
  { name: 'i18n zh 标签', find: new RegExp(`^${IND}bubbleOpacity: "气泡不透明度",$`, 'm'), repl: '$1bubbleOpacity: "气泡不透明度",\n$1bubbleBlur: "气泡模糊程度",', once: true },
  { name: 'i18n zh 提示', find: new RegExp(`^${IND}bubbleOpacityHint: "调节支持气泡 alpha 的皮肤消息气泡，例如鲸鱼妈妈。",$`, 'm'), repl: '$1bubbleOpacityHint: "调节支持气泡 alpha 的皮肤消息气泡，例如鲸鱼妈妈。",\n$1bubbleBlurHint: "本皮肤所有气泡底的毛玻璃强度（0 = 不模糊，默认 10px）。",', once: true },
  { name: 'i18n en 标签', find: new RegExp(`^${IND}bubbleOpacity: "Bubble opacity",$`, 'm'), repl: '$1bubbleOpacity: "Bubble opacity",\n$1bubbleBlur: "Bubble blur",', once: true },
  { name: 'i18n en 提示', find: new RegExp(`^${IND}bubbleOpacityHint: "Controls translucent message bubbles for skins that expose bubble alpha, such as Whale Mom.",$`, 'm'), repl: '$1bubbleOpacityHint: "Controls translucent message bubbles for skins that expose bubble alpha, such as Whale Mom.",\n$1bubbleBlurHint: "Backdrop blur strength for this skin\'s bubble surfaces (0 = sharp, default 10px).",', once: true }
];

// ── 宿主补丁 ──────────────────────────────────────────────────────────────
const HOST_PATCHES = [
  { name: '默认值 +bubbleBlur', find: new RegExp(`^${IND}bubbleOpacity: 50$`, 'm'), repl: '$1bubbleOpacity: 50,\n$1bubbleBlur: 10', once: true },
  { name: 'RANGES +bubbleBlur', find: new RegExp(`^${IND}bubbleOpacity: \\[0, 100\\]$`, 'm'), repl: '$1bubbleOpacity: [0, 100],\n$1bubbleBlur: [0, 20]', once: true },
  {
    name: 'zod schema +bubbleBlur',
    find: new RegExp(`^${IND}bubbleOpacity: z\\.number\\(\\)\\.min\\(0\\)\\.max\\(100\\)\\.step\\(5\\)\\.default\\(SKIN_BACKGROUND_DEFAULTS\\.bubbleOpacity\\)$`, 'm'),
    repl: '$&,\n$1bubbleBlur: z.number().min(0).max(20).step(1).default(SKIN_BACKGROUND_DEFAULTS.bubbleBlur)',
    once: true
  }
];

/**
 * 「是否已打」的判据。
 *
 * ⚠️ 不能再用 `BUBBLE_BLUR_VAR`：上游 0.3.22 起自己就带这个标识符（原版 bundle 里
 * 有 4 处），拿它判断会**永远显示「已打」**。改用本脚本独有的中文注释标记。
 */
const MARK = '本地补丁（skin-center-bubble-blur.mjs）';

function run(file, patches, label) {
  if (!existsSync(file)) { console.log(`  [跳过] 不存在: ${file}`); return; }
  const backup = file + '.orig-bubble-blur';
  let src = readFileSync(file, 'utf8');
  const patched = src.includes(MARK);

  if (check) { console.log(`  [${patched ? '已打' : '未打'}] ${label}: ${file}`); return; }

  // 还原 = 直接把备份覆盖回去（repl 里含 $1 组，反向替换不可靠）
  if (revert) {
    if (!existsSync(backup)) { console.log(`  [无备份] ${label}: ${file}`); return; }
    copyFileSync(backup, file);
    console.log(`  ✓ 已从备份还原 ${label}: ${file}`);
    return;
  }

  if (patched) { console.log(`  [已打] ${label}（幂等跳过）`); return; }
  if (!existsSync(backup)) { copyFileSync(file, backup); console.log(`  备份 → ${backup}`); }

  let miss = [];
  for (const p of patches) {
    const before = src;
    if (typeof p.find === 'string') src = src.split(p.find).join(p.repl);
    else src = src.replace(p.find, p.repl);
    const n = before === src ? 0 : 1;
    if (n === 0) miss.push(p.name);
    else console.log(`  ✓ ${p.name}`);
  }
  writeFileSync(file, src, 'utf8');
  console.log(`  已写入 ${label}: ${file}`);
  if (miss.length) console.log(`  ⚠️ 未命中（可能已被上游改动）：${miss.join(' / ')}`);
}

console.log(`${check ? '检查' : revert ? '还原' : '应用'} 皮肤中心「气泡模糊」补丁  (DSH_HOME=${home}, profile=${profile})`);
console.log('客户端（聚合包 + 独立包）：');
for (const f of CLIENT_FILES) run(f, CLIENT_PATCHES, 'client');
console.log('宿主（schema / 默认值 / 范围）：');
for (const f of HOST_FILES) run(f, HOST_PATCHES, 'host');
console.log(revert ? '完成。' : '完成。客户端重新加载页面即生效；**宿主改动需重启后端**才会持久化 bubbleBlur。');
