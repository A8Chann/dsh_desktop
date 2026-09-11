---
name: cost-meter-commandcode
description: >
  dsh-cost-meter 内置的 CommandCode Coding Plan 额度面板的启用方式、配置存储陷阱、端点解析与验证套路。
whenToUse: >
  dsh-cost-meter 的 CommandCode 额度面板、`ledger.json` 配置、额度端点解析
---

# dsh-cost-meter 的 CommandCode 额度面板

> 记录时间：2026-09-11。

## 面板早已内置

`dsh-cost-meter@1.7.19` 起 `commandcode` 就是第 7 家 Coding Plan：

- `lib/coding-plans.js` 的 `CODING_PLAN_PROVIDERS` / `CODING_PLAN_ENDPOINTS` / `parseCommandCodeCredits`
- `store.js` 的默认值与 `SECRET_REF_MAP`
- `client.js` 的注册表与短标签 `CC`

**无需开发**。默认 `enabled:false, display:'settings'`，所以侧边栏看不到。

## 启用方式（推荐走 UI）

设置 → 费用 → Coding Plan → **CommandCode** → 勾「启用额度查询」+ 显示位置选「主页面侧边栏 / 两者」。Key 自动从 DSH 凭据库的 `COMMANDCODE_API_KEY` 发现（`SECRET_REF_MAP` 里 `'codingPlans.commandcode' → 'COMMANDCODE_API_KEY'`），不必手填。

## ⚠️ 不要在实例运行时改它的配置文件

全部配置在 `$DSH_HOME/storages/cost-meter/ledger.json`（**不在** `settings.yaml`，也没有 `dsh-cost-meter` 设置段）。宿主在 `Store.flush()` 时把**内存里的 config** 原子写回该文件（`store.js:2103`），而本会话每产生一次用量就会触发 flush → **手改的 `enabled` 会在 1~2 分钟内被覆盖回 `false`**（实测 02:56 改、02:58 就被抹掉）。要改文件只能趁进程没在跑。

## 端点与解析实测

2026-09-11，真实 Key：

```
GET https://api.commandcode.ai/alpha/billing/credits
Headers: Bearer + x-command-code-version + x-cli-environment: production
→ {credits:{monthlyCredits}, windowLimits:{fiveHour:{used,cap,resetAt}, weekly:{…}}}
```

`parseCommandCodeCredits` 输出 `fiveHour.percent 1.7 / weekly.percent 0.7 / monthly.text 余额 $69.76`，与 OpenCode Go 卡片同构（5h / 周 + 月度余额）。

## 验证套路（无需 UI）

直接跑插件自己的解析器对真实响应做断言：

```js
const cp = await import('file:///…/dsh-cost-meter/lib/coding-plans.js')
cp.parseCommandCodeCredits(payload, { now: Date.now() })
```

## 注意

`CODING_PLAN_PARSERS` **未导出**，解析器是按名字逐个导出的（`parseCommandCodeCredits` 等）；`CODING_PLAN_ENDPOINTS` / `CODING_PLAN_PROVIDERS` / `CODING_PLAN_PROVIDER_IDS` 才在导出表里。
