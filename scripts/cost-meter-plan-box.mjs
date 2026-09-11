#!/usr/bin/env node
/**
 * cost-meter-plan-box — 让 dsh-cost-meter 侧边栏「Coding Plan」图框的正文
 * 与 OpenCode Go 图框同款（README 声称「侧边栏卡片与 Go 额度同款」，实际不是）。
 *
 * 差异（1.7.19 / 1.7.20 都一样）：
 *   Go 图框 (Vt)       : cm-bbox-head(标题 + 主窗口百分比) → 全宽进度条
 *                        → cm-bbox-line(次要窗口) → cm-bbox-line(重置时间)
 *   CodingPlan 图框 (xn): cm-mm-title(标题独占一行)
 *                        → 每个窗口一行「标签 + 短进度条 + 百分比」，且不显示重置时间
 *
 * 本脚本改两处：
 *   ① 正文版式换成 Go 同款；
 *   ② 外层 className 去掉 `cm-mm`（它的 `gap:4px` 会覆盖 `.cm-bbox` 的 `gap:6px`，
 *      去掉后行距与 Go 图框完全一致；compact / simple 模式下仍走各自原有规则）。
 * 只改客户端 bundle，宿主半边无需改动。
 *
 * 用法：
 *   node scripts/cost-meter-plan-box.mjs            # 应用（幂等）
 *   node scripts/cost-meter-plan-box.mjs --check    # 查看状态
 *   node scripts/cost-meter-plan-box.mjs --revert   # 从 *.orig-plan-box 还原
 *
 * ⚠️ 插件升级（dsh-cost-meter）会覆盖 lib/client.js → 升级后重跑本脚本即可。
 * ⚠️ 改完需要重新加载内容页（标题栏聚焦后 F5）。
 */
import { existsSync, readFileSync, writeFileSync, copyFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { homedir } from 'node:os';
import vm from 'node:vm';

/**
 * ① 正文：被替换的最小片段（`p` 是逐窗口的行元素数组）。
 * 替换后使用 Go 图框的版式。作用域内可用：v=窗口条目数组、g=取整数百分比、
 * h=百分比格式、ce=百分比→{width,label}、bt=窗口短标签、i=文案函数、b=厂商元数据、
 * e=createElement、q=Fragment。局部名（B/U/F/Z/K/W/Y/J/ee/tt）刻意避开 xn 作用域已有的
 * s,o,n,a,r,l,i,d,b,u,v,g,h,p,x,N,P,M,w，避免遮蔽。
 */
const BODY_FROM = 'e(q,null,e("div",{className:"cm-mm-title"},i(b.labelKey)),...p)';
const BODY_TO =
  'e(q,null,...(()=>{' +
  'const B=v[0][0],U=v[0][1],F=g(U),Z=F===null?null:ce(F,h),' +
  'K=F===null?(U!==null&&typeof U?.text==="string"?U.text:"—"):(Z.label===null?"—":Z.label+"%"),' +
  'W=v.slice(1).map(([Y,J])=>{const ee=g(J);' +
  'return ee===null?(J!==null&&typeof J?.text==="string"?J.text:"—"):bt(Y,i)+" "+ce(ee,h).label+"%"}),' +
  'tt=U!==null&&typeof U?.resetsAt==="string"&&U.resetsAt.length>0' +
  '?i("goResetAt",{time:new Date(U.resetsAt).toLocaleString()}):"";' +
  'return[' +
  'e("div",{className:"cm-bbox-head"},' +
  'e("span",{className:"cm-bbox-label"},i(b.labelKey)+" "+bt(B,i)),' +
  'e("span",{className:"cm-bbox-pct cm-num"},K)),' +
  'F===null?null:e("div",{className:"cm-bbox-bar"},' +
  'e("div",{className:"cm-bbox-fill",style:{width:Z.width+"%"}})),' +
  'W.length>0?e("div",{className:"cm-bbox-line cm-num"},W.join(" · ")):null,' +
  'tt?e("div",{className:"cm-bbox-line"},tt):null' +
  ']})())';

/**
 * ② 外层 className：去掉 `cm-mm`。
 * 锚点带 `+(M==="ok"` 是为与 vn / Nn 两个同样写 `"cm-bbox cm-mm clickable"` 的
 * 图框区分开（`M` 只在 xn 作用域内）。
 */
const CLASS_FROM = '"cm-bbox cm-mm clickable"+(M==="ok"?"":" "+M)';
const CLASS_TO = '"cm-bbox clickable"+(M==="ok"?"":" "+M)';

const REPLACEMENTS = [
  { from: BODY_FROM, to: BODY_TO, label: '正文版式' },
  { from: CLASS_FROM, to: CLASS_TO, label: '外层间距类' },
];

function dshHome() {
  return process.env.DSH_HOME && process.env.DSH_HOME.length > 0
    ? process.env.DSH_HOME
    : join(homedir(), '.dsh');
}

/** 收集所有 profile 下已安装的 cost-meter 客户端 bundle。 */
function targets() {
  const profilesDir = join(dshHome(), 'profiles');
  const found = [];
  if (!existsSync(profilesDir)) return found;
  for (const name of readdirSync(profilesDir)) {
    const file = join(profilesDir, name, 'node_modules', 'dsh-cost-meter', 'lib', 'client.js');
    if (existsSync(file)) found.push({ profile: name, file });
  }
  return found;
}

function countOf(src, needle) {
  return src.split(needle).length - 1;
}

/** original = 两处都还是原样；patched = 两处都已替换；其余为 unknown。 */
function statusOf(src) {
  const from = REPLACEMENTS.filter((r) => countOf(src, r.from) === 1).length;
  const to = REPLACEMENTS.filter((r) => countOf(src, r.to) >= 1).length;
  if (to === REPLACEMENTS.length) return 'patched';
  if (from === REPLACEMENTS.length) return 'original';
  return 'unknown';
}

function syntaxOk(src) {
  try {
    new vm.Script(src, { filename: 'client.js' });
    return true;
  } catch (error) {
    console.error('  ✗ 语法校验失败：', error.message);
    return false;
  }
}

const mode = process.argv.includes('--check')
  ? 'check'
  : process.argv.includes('--revert')
    ? 'revert'
    : 'apply';

let failed = false;

for (const { profile, file } of targets()) {
  const backup = `${file}.orig-plan-box`;
  const src = readFileSync(file, 'utf8');
  const state = statusOf(src);
  console.log(`[${profile}] ${file}`);
  console.log(`  当前状态：${state}`);

  if (mode === 'check') {
    console.log(
      state === 'patched'
        ? '  ✓ 已应用（Coding Plan 图框为 Go 同款版式）'
        : state === 'original'
          ? '  · 未应用（原始版式）'
          : '  ⚠️ 无法识别：文件可能已被上游改版或其它补丁修改'
    );
    continue;
  }

  if (mode === 'revert') {
    if (!existsSync(backup)) {
      console.log('  ⚠️ 没有备份文件，无法还原');
      failed = true;
      continue;
    }
    copyFileSync(backup, file);
    console.log('  ✓ 已还原自 ' + backup);
    continue;
  }

  // apply
  if (state === 'patched') {
    console.log('  ✓ 已是补丁状态，跳过（幂等）');
    continue;
  }
  if (state !== 'original') {
    console.log('  ✗ 目标片段不完整，未修改（上游可能已改版，请人工核对）');
    failed = true;
    continue;
  }

  let next = src;
  const applied = [];
  let abort = false;
  for (const r of REPLACEMENTS) {
    if (countOf(next, r.from) !== 1) {
      console.log(`  ✗ 「${r.label}」目标片段出现次数不为 1，出于安全未修改`);
      abort = true;
      break;
    }
    next = next.replace(r.from, r.to);
    applied.push(r.label);
  }
  if (abort) {
    failed = true;
    continue;
  }
  if (!syntaxOk(next)) {
    console.log('  ✗ 补丁后语法校验未通过，已放弃写入');
    failed = true;
    continue;
  }
  if (!existsSync(backup)) {
    copyFileSync(file, backup);
    console.log('  · 已备份 → ' + backup);
  }
  writeFileSync(file, next, 'utf8');
  console.log(`  ✓ 已应用补丁（${applied.join(' + ')}）；重新加载内容页生效`);
}

if (failed) process.exitCode = 1;
