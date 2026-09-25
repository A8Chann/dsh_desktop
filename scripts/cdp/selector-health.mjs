#!/usr/bin/env node
/**
 * 皮肤选择器体检：找出 patches.css 里「钉了哈希段」的类名子串，逐个在**当前页面**上核对是否还有命中。
 *
 *   node scripts/cdp/selector-health.mjs                    # 体检
 *   node scripts/cdp/selector-health.mjs --save             # 存一份基线（升级前跑）
 *   node scripts/cdp/selector-health.mjs --compare          # 与基线比对（升级后跑）
 *   node scripts/cdp/selector-health.mjs [patches.css] [--save|--compare] [baseline.json]
 *
 * 为什么需要：dsh / 插件升级后，CSS-module 的哈希段会变（`_card_1b2ny_13` → `_card_178vx_13`），
 * 钉死整串的规则会**静默失效**（样式悄悄回到上游默认值），肉眼往往过很久才发现
 * （本机 2026-09-24 的「hover 浮卡又黑了」就是这么来的）。
 *
 * 单次体检的判定：
 *   OK          选择器在当前页面有命中
 *   疑似漂移     没命中，但页面上存在「同局部名 + 不同哈希段」的类 → 极可能哈希变了
 *   页面未出现   两者都没命中 → 该元素当前页面没有（换状态/换个会话再跑，别急着改）
 *
 * ⭐ **真正的用法是「基线 + 比对」**：升级前 `--save`，升级后 `--compare`。
 *    「升级前有命中、升级后 0 命中」= 明确回归（多半是哈希漂移），直接列出来，不用靠运气发现。
 *    页面状态差异（工具行 / 展开正文 / 流式 / 设置面板…）会制造噪声，所以比对表里也标注了基线值。
 */
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { connect, dshPage, evaluate } from './cdp.mjs';

const argv = process.argv.slice(2);
const flags = argv.filter((a) => a.startsWith('--'));
const args = argv.filter((a) => !a.startsWith('--'));
const cssPath = args[0] ?? `${process.env.USERPROFILE}\\.dsh\\skins\\blue-fantasy\\patches.css`;
const baselinePath = args[1] ?? 'assets/skins/blue-fantasy/selector-baseline.json';
const mode = flags.includes('--save') ? 'save' : flags.includes('--compare') ? 'compare' : 'check';

const css = readFileSync(cssPath, 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
const tokens = [...new Set([...css.matchAll(/\[class[\*\^]=["']([^"']+)["']\]/g)].map((m) => m[1]))].sort();

/** 语义前缀白名单（插件自己起的稳定前缀，不是构建哈希） */
const STABLE_PREFIX = /^(cm-|md-|dsh-)/;

/** 哈希形态：末段是局部名，前一段是 3-8 位的哈希段（大小写、数字、连字符都可能出现） */
const hashShape = (token) => {
  const segs = token.replace(/^[-_]/, '').split('_');
  if (segs.length >= 3 && /^[A-Za-z0-9-]{3,8}$/.test(segs[segs.length - 2])) {
    return { local: segs[segs.length - 1], family: segs.slice(0, -1).join('_') };
  }
  return null;
};

const candidates = tokens.filter(
  (t) => !STABLE_PREFIX.test(t) && !/^(actions|bubble|markdown|panel|callRow|userStack|tableFill|tableScroll|entryRow)$/.test(t),
);

const page = await dshPage();
const { send, close } = await connect(page.webSocketDebuggerUrl);
await send('Runtime.enable');

const probe = await evaluate(
  send,
  `(() => {
    const tokens = ${JSON.stringify(candidates)};
    const classes = new Set();
    for (const el of document.querySelectorAll('body *')) for (const c of el.classList) classes.add(c);
    const all = [...classes];
    return JSON.stringify(tokens.map((t) => {
      const exact = document.querySelectorAll('[class*="' + t + '"]').length;
      const local = t.replace(/^[-_]/, '').split('_').pop();
      const sameLocal = all.filter((c) => c !== t && new RegExp('^[-_]?[A-Za-z0-9]{3,8}[-_][A-Za-z0-9]{2,5}_' + local + '$').test(c));
      return { token: t, exact, local, sameLocal: sameLocal.slice(0, 4) };
    }));
  })()`,
);

const rows = JSON.parse(probe);
const counts = Object.fromEntries(rows.map((r) => [r.token, r.exact]));
const pad = (s, n) => String(s).padEnd(n);

if (mode === 'save') {
  const payload = { savedAt: new Date().toISOString(), page: page.url.replace(/\?.*$/, ''), counts };
  writeFileSync(baselinePath, JSON.stringify(payload, null, 2) + '\n');
  console.log(`基线已存 → ${baselinePath}（${Object.keys(counts).length} 个选择器）`);
  console.log('提示：这是「升级前」的参照，升级 dsh / 插件后再跑 --compare。');
  close();
  process.exit(0);
}

if (mode === 'compare') {
  if (!existsSync(baselinePath)) { console.error(`找不到基线：${baselinePath}（先在升级前跑 --save）`); close(); process.exit(2); }
  const base = JSON.parse(readFileSync(baselinePath, 'utf8'));
  const regressed = [], improved = [];
  for (const [token, now] of Object.entries(counts)) {
    const before = base.counts?.[token];
    if (before === undefined) continue;
    if (before > 0 && now === 0) regressed.push({ token, before, now });
    if (before === 0 && now > 0) improved.push({ token, before, now });
  }
  console.log(`与基线比对（基线存于 ${base.savedAt}）`);
  console.log('-'.repeat(80));
  if (!regressed.length) console.log('✅ 没有「基线有、现在 0」的选择器。');
  else {
    console.log(`⚠️ 疑似回归 / 漂移（${regressed.length} 个）—— 这些升级前有命中、现在为 0：`);
    for (const r of regressed) console.log(`   ${pad(r.token, 26)} ${r.before} → 0`);
    console.log('\n   处理：到 node_modules 里搜「同局部名的其它类」（如 o3BgMG_root → 搜 _root / o3BgMG_），');
    console.log('         确认新哈希后更新 patches.css；能换成语义锚点（data-*/行内变量/:has）的就换掉。');
  }
  if (improved.length) {
    console.log(`\nℹ️ 基线为 0、现在有命中的（${improved.length} 个，通常是页面状态差异，非问题）：`);
    for (const r of improved) console.log(`   ${pad(r.token, 26)} 0 → ${r.now}`);
  }
  close();
  process.exit(regressed.length ? 1 : 0);
}

let drifted = 0, absent = 0, ok = 0;
console.log('选择器体检（页面: ' + page.url.replace(/\?.*$/, '') + '）');
console.log(pad('结果', 12) + pad('命中', 5) + pad('选择器', 26) + '同局部名的其它类');
console.log('-'.repeat(100));
for (const r of rows) {
  const shape = hashShape(r.token);
  let verdict, note = '';
  if (r.exact > 0) { verdict = 'OK'; ok += 1; }
  else if (r.sameLocal.length) { verdict = '疑似漂移'; drifted += 1; note = r.sameLocal.join(' '); }
  else { verdict = '页面未出现'; absent += 1; note = shape ? '（是哈希形态，换个页面状态复查）' : '（非哈希形态/语义词）'; }
  console.log(pad(verdict, 12) + pad(r.exact, 5) + pad(r.token, 26) + note);
}
console.log('-'.repeat(100));
console.log(`OK ${ok} / 疑似漂移 ${drifted} / 页面未出现 ${absent}（共 ${rows.length}）`);
console.log('\n💡 「页面未出现」多半只是当前页面没这类元素。要判断有没有真回归，请用 --save / --compare 基线比对。');
close();

