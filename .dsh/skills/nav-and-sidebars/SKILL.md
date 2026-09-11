---
name: nav-and-sidebars
description: >
  改顶部导航栏 / 左右侧边栏底色与面板渲染层级（display:contents 宿主坑）。
---

# 顶部导航栏 / 右侧侧边栏 / 左侧栏底色规则

## 左侧栏底色（基准色）

`[data-pane="sidebar"]` 的 computed `backgroundColor` = **`rgba(242,245,250,.75)`**（实测值）。顶部导航栏与右侧栏面板统一用这个色 + `.75`：

```css
[data-slot="main.conversation"] header,
[data-slot="rightbar.session"] > *,
[class*="rightbarCol"] [class*="panel"] {
  background: rgba(242,245,250,.75);
}
```

顶部另有 1px 分隔线。深色主题：`rgba(16,22,42,.75)`。

## 导航栏内部各项：**不加任何背景**

（会话名 / 模式选择 / 对话 / 轨迹 / 上下文）—— 最终定稿为「不加任何背景」。

历程（三轮反转，记下来免得再绕）：
1. 按要求加贴合内容宽度的模糊底；
2. 用户反馈「文字底下的模糊很怪」→ 去掉 `backdrop-filter` 只留底色；
3. 用户明确「那几个字不要背景了」→ **整条规则删除**（背景 / 圆角 / 内边距 / 宽度都不加），恢复官方原始间距与下划线指示器。

实测：会话名 102x28、模式 71x22、tab 26/26/39，全部 `bg=rgba(0,0,0,0)`、`bf=none`、`padding 0`。

若以后又要小底，选择器用语义属性：
- 会话名 = `header nav[aria-label="会话层级"]`
- tab = `header [role="tablist"] button[role="tab"]`
- 模式选择没有稳定钩子，只能用 `[class*="headerActions"]`

## 右侧侧边栏

**是官方的 `@deepseek-ai/dsh-client-ui-sidebar-right`**（`rightbarCol` / `rightbar.session` / `P3OORG_panel` 都在它里面）。`dsh-better-sidebar` 只是通过 `dsh.client.inject` 往官方右侧栏里塞「文件/终端/浏览器」内容，**右侧栏本身不是它的**。

### 渲染层级

```
[class*="rightbarCol"]      （列）
  └ [data-slot="rightbar"]        （display:contents，0×0）
      └ [data-slot="rightbar.session"]  （同样 0×0）
          └ [class*="panel"]             （真正画出的面板，position:absolute）
```

**给前两个宿主上底等于没上**（第一版踩在这，用户反馈「侧边栏背景没生效」）。必须写到 `.panel` 那一层。

### 判断面板属于谁

直接在 `node_modules` 里：
```powershell
Select-String -Pattern "<哈希前缀>|<slot名>"
```
本次 `P3OORG_panel` / `rightbar.session` 只命中官方包，better-sidebar 里一次都没有。
