# 原型对齐 · 阶段四 B「灵动岛 / 感应区 / 提醒卡片 / 置顶浮窗」设计

**日期**：2026-09-12
**前置**：阶段四 A `2026-09-12-prototype-realign-phase4a-overlays-design.md`（D22–D28）；阶段三 D15–D21；阶段二 D10
**原型**：`docs/index.html` `#hotzone`（71）、`#dynamicIsland`（73–79）、灵动岛页设置行（218–233）、`#pinnedWindow`（449–457）、`#reminderCard`（459–478）；`docs/app.js` 灵动岛 276–360、提醒岛内呈现 496–513、设置桥接 3442–3476、置顶 3149–3168、提醒 1348–1367；`docs/styles.css` 254–313、1159–1193；生成层 `src/styles/components.css` 已含 `#hotzone`、`#dynamicIsland` / `.di-*` / `.di-glow`、`#pinnedWindow`、`#reminderCard` 全部规则

## 1. 范围

对齐四个独立小窗口：灵动岛（DOM 结构、逐条停留轮播、数据口径含子任务与播放范围、四个新设置项、手柄拖拽、提醒岛内呈现、面板展开淡出、全屏自动隐藏）、感应区（指示器改用令牌）、提醒卡片（入退场动效）、置顶浮窗（双击编辑回写）；修复 `settings_save` 同步建窗死锁；删除 `island.css` 与 `extensions.css` §8；顺带落实 4A 遗留的主窗口删除确认兜底。

**不在 4B**：浮窗启动台（4C）；灵动岛悬停语义（D24 保留现状）；感应区阈值（D19 保留 3 秒）。

## 2. 决策记录（本阶段新增，均已确认）

| # | 问题 | 采用 | 理由 |
|---|---|---|---|
| D29 | 主窗口删除确认兜底 | 主窗口四个视图也改为「卡片下方」（与面板一致）；`anchorBeside` 补 `above` 分支，翻到上方时箭头朝下 | 实测主窗口卡片横贯整行，居中兜底会盖住正文 |
| D30 | 全屏自动隐藏探测 | Windows `SHQueryUserNotificationState`：返回 `QUNS_RUNNING_D3D_FULL_SCREEN` / `QUNS_PRESENTATION_MODE` / `QUNS_BUSY` 时视为全屏，隐藏灵动岛；恢复时重新显示 | 官方「免打扰」语义，代码最少 |
| D31 | 置顶浮窗双击编辑 | 双击进入编辑态（文本域），Ctrl+Enter 或失焦保存，按 kind 走 `notes.save` / `todos.save` / `clipboard.update`，toast「已同步回数据库 ✔」 | 原型行为 |
| D32 | 灵动岛参数范围（承阶段四拍板） | 宽 200–480 / 高 32–56 / 停留 1–10 秒 / 默认 320×36·3 秒；存量设置不迁移，只改默认与钳制；透明度仍只透底色 | 对齐原型；文字可读性优先 |
| D33 | 灵动岛悬停（承 D24） | 保留「悬停撑高显示详情、点击开面板」；新增「悬停穿透」开关为真时不撑高、不暂停轮播 | 用户选择 |
| D34 | `settings_save` 死锁修法 | 命令改 `pub async fn`（与 `pin_create` / `launcher_show` 同款） | 最小改动、与既有约定一致 |

## 3. 与原型的差异处理表

| # | 项 | 原型 | 现状 | 做法 |
|---|---|---|---|---|
| **灵动岛** | | | | |
| 1 | DOM | `#dynamicIsland.glass` > `.di-ticker` > `.di-track` > `.di-item`(`.di-dot.{high\|medium\|low\|idle}` + `.di-text`(+`.di-date`) + `.di-time(.ovd)`)；`.di-alert`；`.di-resizer` | `.island` > `.island-line` > `.island-row`（自建） | 就地重写模板；删 `src/styles/island.css` 与 `extensions.css` §8 `.island` 行；`island.ts` 不再引入 `island.css` |
| 2 | 轮播 | 每条停留 `stay` 秒 → `translateY(-i*h)` 0.45s ease → 末尾克隆第一条，滚到克隆后 480ms 无过渡复位；仅一条静止 | `setInterval` + `Transition` out-in 0.24s | 就地实现 `diStep`（§4.1） |
| 3 | 数据口径 | 未完成顶级 + 子任务（文案 `父 / 子`）；scope=today 仅当天，all=全部未完成；逾期优先再按 dueTime | 仅顶级、今天或逾期、按 due 升序 | 共享件：`pickIslandTodos(todos, scope, today)` 替换 `pickTodayTodos`；主窗口灵动岛页预览同源 |
| 4 | 空态 | `.di-dot.idle` + `.di-text.dim`「今日待办已全部完成 🎉」/「没有未完成待办 🎉」 | 「今天没有待办 · 点此新建」 | 就地按 scope 二选一 |
| 5 | 悬停 | 100ms 开面板 + 暂停轮播；`islandPassHover` 时都跳过 | 撑高详情；点击开面板 | D33：保留撑高，加 `island_pass_hover` 守卫（后端 watcher 读设置为真时不发 `ISLAND_HOVER`） |
| 6 | 手柄拖拽 | `.di-resizer` 右下 14×14，hover 可见；宽 `Δx*2` 对称、高 `Δy`，钳制后落库 | 无 | 就地 + 后端 `island_resize(width, height)` 命令（`place_island` + 写 `island_width/height`，不经 `settings_save`）；穿透时手柄不可用（原型同） |
| 7 | 面板展开淡出 | `.di-fade`（opacity 0 + pointer-events none） | 无 | 就地：订阅 `panelShown` / `panelHidden`（后端已全局广播） |
| 8 | 提醒岛内呈现 | `.di-alert`：⏰ 脉冲 + 「提醒：内容」金色，6s 或点击恢复；岛隐藏时不呈现 | 无 | 就地：订阅 `reminderFired`（payload 为 todo id）→ 从 `useTodos` 取内容；穿透时靠 6s 自动 |
| 9 | 流光边框 | `.di-glow` conic 旋转（生成层已有） | 无 | 后端 `island_glow: bool`；前端 class 切换 |
| 10 | 全屏自动隐藏 | 仅持久化开关 | 无 | D30：后端 `island_auto_hide: bool`（默认 true）+ `platform::foreground_is_fullscreen()` + watcher 每 ~1s 探测翻转时 `hide()/show()` |
| 11 | 播放范围 | `islandScope` today/all | 无 | 后端 `island_scope: String`（today/all，默认 today）；灵动岛页补 select |
| 12 | 参数范围 | 200–480 / 32–56 / 1–10 / 320×36·3 | 200–800 / 28–72 / 2–30 / 360×36·4 | D32：`island_clamp` 与前端 `ISLAND_LIMITS` 改；默认改；存量不迁移 |
| 13 | 透明度 | 整岛 opacity | `--island-alpha` 只透底色 | D32 保留（差异接受） |
| 14 | 点击穿透 | 只禁 alert/resizer 的 pointer-events | 整窗 `set_ignore_cursor_events` | 保留（能力等价） |
| 15 | 灵动岛页状态行 | 含「范围：…」「悬停穿透」 | 缺 | 就地补 |
| 16 | 灵动岛页设置 | 8 项 | 缺播放范围 / 悬停穿透 / 流光 / 全屏隐藏 | 补 4 行（原型顺序：启用 → 停留 → 播放范围 → 透明度 → 点击穿透 → 悬停穿透 → 流光 → 全屏隐藏；宽 / 高 / 插件为项目扩展，排在其后） |
| 17 | `settings_save` | — | 同步命令 → 灵动岛关→开时 `create_island` 死锁 | D34 改 async |
| **感应区** | | | | |
| 18 | 指示器 | 无 | 3 秒 + 指示器，固定深色底 + §8 钉浅色令牌 | D19 保留 3 秒；指示器底色 / 边框 / 文字改用 `--glass-bg` / `--glass-border` / `--text` 令牌；删 §8 `.hotzone-indicator` 行 |
| **提醒卡片** | | | | |
| 19 | 入 / 退场 | x:60 opacity 0 → 0.3s back.out(1.5)；退场 x:60 0.2s | 无 | `enter/exit` 横向 60px，`slow` / `base` 档；退场播完再 `reminderClose` |
| 20 | 与灵动岛联动 | `showReminder` 同步 `showIslandAlert` | 无 | 由 #8 覆盖（事件已广播） |
| 21 | toast | 「已关闭提醒」/「已设置下次提醒」 | 无处显示 | 保留（差异接受，窗口随即关闭） |
| 22 | 「已完成」选项 | 无 | 有 | 保留（超集） |
| **置顶浮窗** | | | | |
| 23 | 双击编辑回写 | prompt 编辑 → 回写 + toast | 只放大窗口 | D31：编辑态 `textarea`（复用 `pinSetEditing` 放大），Ctrl+Enter / 失焦保存；Esc 取消 |
| 24 | 整卡拖动 | 是 | 标题栏拖动 | 保留（drag-region 会吞点击） |
| 25 | 宽度 | 220 | 230 | 改 220（`windows.rs` 常量） |

## 4. 设计

### 4.1 灵动岛前端（`src/windows/Island/IslandApp.vue`、`src/island-plugins/`）

- 模板（原型结构，`.glass` 由生成层提供毛玻璃与圆角）：

```vue
<div id="dynamicIsland" class="glass" :class="{ 'di-fade': panelOpen, 'di-pass-click': settings.island_click_through, 'di-glow': settings.island_glow, expanded }" :style="{ '--island-alpha': alpha, '--cap-h': capH + 'px' }">
  <div class="di-ticker"><div ref="track" class="di-track" :style="trackStyle">
    <div v-for="(row, i) in rows" :key="row.key + ':' + i" class="di-item" :style="{ height: itemH + 'px' }">
      <span class="di-dot" :class="row.dotClass" /><span class="di-text" :class="{ dim: row.dim }"><span v-if="row.date" class="di-date">{{ row.date }}</span>{{ row.text }}</span><span v-if="row.time" class="di-time" :class="{ ovd: row.overdue }">{{ row.time }}</span>
    </div>
  </div></div>
  <div v-if="alert" class="di-alert" @click="hideAlert"><span class="di-alert-ico">⏰</span><span class="di-alert-text">提醒：{{ alert }}</span></div>
  <div v-if="!settings.island_click_through" class="di-resizer" @mousedown.prevent="startResize" />
  <Transition name="island-fade"><div v-if="expanded" class="island-expanded">…（现有详情组件不变）</div></Transition>
</div>
```

- `rows`：`items` 映射为行；`items.length > 1` 时末尾追加第一条副本（key 加后缀）。
- 轮播：`diIndex` / `timer`（`setTimeout` 链）；`step()`：`diIndex++`，`trackStyle = { transition: 'transform .45s ease', transform: translateY(-diIndex*itemH) }`；到副本后 480ms 设 `transition:'none'`、`diIndex=0`；`stay = clamp(island_cycle_seconds, 1, 10)`；`expanded` / `alert` / `items.length<=1` 时不步进；`items` 变化时重置。
- 数据：`island-plugins/today-todos.ts` 的 `useItems` 改用 `pickIslandTodos(todos, settings.island_scope)`；`IslandItem` 增 `date?: string`（非当天 `MM-DD`）、`overdue?: boolean`、`priority?`；`dotClass` 由 priority 映射 `high/medium/low`，空态 `idle`。文案子任务 `父内容 / 子内容`。
- `pickIslandTodos(todos, scope, today = todayKey())` 放 `src/utils/island.ts`（纯函数 + `node:test`：today/all 口径、子任务文案、逾期优先排序、空集）。
- 悬停：`islandHover` 事件处理前加 `if (settings.island_pass_hover) return`（后端也不发，双保险）。
- 提醒：`onAppEvent<string>(AppEvents.reminderFired, id => { const t = todos.find(...); if (t) showAlert(t.content) })`；`showAlert` 停轮播、6s 后 `hideAlert` 恢复。
- 面板联动：`panelShown → panelOpen = true`；`panelHidden → false`（若隐藏后事件丢失，`islandHover` 翻转时也同步一次：后端 watcher 在面板可见时不发悬停，收到悬停即说明面板已收）。
- 手柄：`startResize` 记录起点与当前宽高；`document` `mousemove` 计算 `w = clamp(w0 + Δx*2, 200, 480)`、`h = clamp(h0 + Δy, 32, 56)` 并即时改 `capH` / `style.width`；`mouseup` → `api.island.resize(w, h)`。
- 删除 `island.css`，`island.ts` 改为只引 `@/styles`；`extensions.css` §8 整节删除；胶囊高度过渡（原 `--cap-h`）与展开态样式（`.island-expanded` 等）迁入 `extensions.css` 新「§10 灵动岛扩展」分节（只含项目扩展：展开详情、`--cap-h` 过渡、点击反馈）。

### 4.2 灵动岛后端

- `domain/models.rs` `Settings` 新增：`island_scope: String`（默认 `"today"`）、`island_pass_hover: bool`（false）、`island_glow: bool`（false）、`island_auto_hide: bool`（true），带 `serde(default)`；`data/settings.rs` 读写四键；`data/mod.rs` v7 迁移只做 `user_version=7`（settings 为 KV 表，新键靠默认值，迁移函数留空但保持链完整，单测一条）。
- `island_clamp`：宽 200–480、高 32–56、cycle 1–10；默认 `default_island_width=320`、`cycle=3`。
- `ISLAND_LIMITS`（前端 `IslandPageView`）同步。
- `island_resize(app, width, height)`：钳制 → 写 `island_width/height`（`store.save_settings` 只改两键）→ `place_island` → emit `SETTINGS_CHANGED`。`ipc::island_resize` 同步命令（不建窗）。
- `hotzone_watcher.rs`：每轮读 `island_pass_hover`（缓存于 `AppState`，`settings_save` 时刷新）为真则不 emit `ISLAND_HOVER`；每 12 轮（~1s）调 `platform::foreground_is_fullscreen()`，与上次不同且 `island_auto_hide` 为真时 `window.hide()/show()`，日志「[island] 全屏 {enter|exit}」。
- `platform.rs`：`foreground_is_fullscreen() -> bool`：Windows `SHQueryUserNotificationState`（`windows-sys` 加 `Win32_UI_Shell`）；非 Windows false。
- `ipc::settings_save` 改 `pub async fn`（D34）；`AppState` 增 `island_pass_hover` 缓存。

### 4.3 感应区

`extensions.css` §6 `.hotzone-indicator`：`background: var(--glass-bg)`、`border-color: var(--glass-border)`、文字 `var(--text)` / `var(--text-dim)`，删固定 `rgba(24,26,40,…)`；§8 整节删除（`.island` 行随 4.1 一起消失）。核对 `--glass-bg` / `--glass-border` 令牌存在（`tokens.css`），不存在则用 `--menu-bg` / `rgba(var(--wsa), .15)`。

### 4.4 提醒卡片

`ReminderApp.vue`：`onMounted` 对 `#reminderCard` `enter({ axis:'x', distance:60, duration:'slow' })`（`outBack(1.5)`：`enter` 默认 outBack(1.6)，差异接受）；`dismiss` / `snooze` 成功后 `await exit({ axis:'x', distance:60, duration:'base' })` 再 `reminderClose`。

### 4.5 置顶浮窗

`PinnedApp.vue`：`editing` 态渲染 `<textarea class="pinned-editor" v-model="draft">`（`extensions.css` §10 一条样式：铺满 body、透明底、无边框），`dblclick` → `editing=true` + `pinSetEditing(label, true)` + 聚焦置末；`Ctrl+Enter` / `blur` → `save()`：note → `api.notes.save({ id, content: draft, tags, editorMode, mindmapData, draft:false })`（需先 `notes.get(id)` 取原字段），todo → `api.todos.save({...原字段, content: draft})`，clip → `api.clipboard.update(id, draft)`；成功 toast「已同步回数据库 ✔」、`editing=false`、`pinSetEditing(false)`；Esc 放弃。`windows.rs` `PIN_WIDTH` 230→220。

### 4.6 4A 遗留（D29）

`anchor.ts`：`placement` 增 `'above'`；below 翻上方时返回 `'above'`，`caretY` 不用；`CardConfirm` 对 `above` 加类 `.above`（`extensions.css` §3 `.cc-caret` 朝下）；`TodosView` / `NotesView` / `ClipsView` / `DayView` 的 `fallback` 改 `below`（`CardConfirm` 默认已是 below，直接删这四处传值）。单测补 `above` 一例。

## 5. 测试

- `src/utils/island.test.ts`：4 例（§4.1）。`anchor.test.ts` +1。
- Rust：v7 迁移单测（版本号推进、幂等）；`island_clamp` 新范围单测（若已有则改断言）。
- 全套：`pnpm typecheck`、`pnpm test`、`pnpm sync:styles --check`、`format:check`、`cargo fmt/check/test`、`vite build`。

## 6. 实机验收（打字机 + 深色）

1. 灵动岛 DOM 与轮播：2 条以上待办时每条停留 3 秒、0.45s 上滚、回环无跳变；1 条静止；空态文案随范围切换。
2. 数据口径：子任务显示「父 / 子」；scope=all 时非当天带 `MM-DD` 徽章；逾期在前且时间红色。
3. 设置即时生效：停留秒数、透明度、宽 / 高、流光、悬停穿透（悬停不撑高）、播放范围；**启用开关关→开不再冻结**（D34）。
4. 手柄拖拽：宽对称伸缩 200–480、高 32–56，松手后库值更新且灵动岛页数值同步。
5. 提醒岛内呈现：到期提醒时岛切换为「⏰ 提醒：…」6s 后恢复；点击立即恢复；提醒卡片同时右上滑入，关闭时右滑退场。
6. 面板展开淡出：呼出面板岛淡出，收起恢复。
7. 全屏自动隐藏：开一个全屏 D3D 应用或 PowerPoint 放映 → 岛隐藏；退出恢复；开关关闭时不隐藏。
8. 感应区指示器在打字机 / 深色下可读，§8 删除后无回归。
9. 置顶浮窗：双击进入编辑、Ctrl+Enter 回写并 toast、主窗口列表刷新；Esc 放弃。
10. 主窗口删除确认落在卡片下方；面板末张卡翻到上方时箭头朝下。

## 7. 提交批次（中文）

1. `feat(settings): 灵动岛四个新设置项（播放范围 / 悬停穿透 / 流光 / 全屏隐藏）、v7 迁移、参数范围对齐原型、settings_save 改 async`
2. `feat(island): island_resize 命令、悬停穿透守卫、全屏探测与自动隐藏`
3. `feat(island): pickIslandTodos 纯函数（子任务 / 播放范围 / 逾期优先）+ 单测`
4. `feat(island): 灵动岛 DOM 与逐条停留轮播对齐原型，提醒岛内呈现、面板展开淡出、手柄拖拽，删除 island.css`
5. `feat(island): 灵动岛页补四个设置项与状态行`
6. `feat(ui): 感应区指示器改用令牌、提醒卡片入退场动效，删除 extensions.css §8`
7. `feat(pinned): 置顶浮窗双击编辑回写`
8. `fix(ui): 主窗口删除确认改卡片下方兜底，anchorBeside 补 above 分支`
9. `docs(design): 阶段四B 实机验收记录`

## 8. 风险

| 风险 | 应对 |
|---|---|
| `.glass` 在灵动岛透明窗内的毛玻璃绘制盖圆角 | 沿用 `window-fit.css` 的 `clip-path: inset(0 round 999px)` 写法 |
| 轮播 `translateY` 与窗口高度 / `--cap-h` 过渡冲突 | 展开态 `v-if` 隐藏 `.di-ticker`（沿用现状 `expanded` 分支） |
| `SHQueryUserNotificationState` 在无 Shell 通知时返回 `QUNS_NOT_PRESENT` | 视为非全屏 |
| 存量用户 `island_width=360` 超出新上限 480？ | 360 在范围内；`cycle=4` 在 1–10 内；无需迁移 |
| 置顶回写笔记会丢标签 / 导图 | 先 `notes.get` 取原字段再保存 |

## 9. 实机结论（2026-09-13 回填，详见 `../plans/2026-09-12-prototype-realign-phase4b-acceptance.md`）

- 轮播与手柄拖拽：**可用**。4 条待办逐条停留 4 秒、0.45s 上滚、回环无跳变；非当天项带日期徽章、逾期时间红色；手柄在展开态可抓，宽按 Δx×2、高按 Δy 并钳制，松手落库；拖拽与提醒点击期间后端点击探测被交互旗标抑制，未误呼面板（修复 `0a0d890` 引入）。
- 全屏探测：Edge F11 全屏触发「[island] 全屏 enter」并隐藏灵动岛，退出后「exit」恢复；1 Hz 探测带来 ≤1 秒翻转延迟，可接受。游戏 / PPT 放映未逐一验证（同一 QUNS 语义）。
- 岛内提醒：提醒触发时胶囊显示「⏰ 提醒：{内容}」6 秒；提醒卡片同时右上滑入、关闭时右滑退场。
- 主题可读性：底色改用各主题都定义的 `--menu-bg` 后，`light` 与打字机下均浅底深字（修复 `bc28ebc`；修前浅色主题下为深字压深底）。
- 补丁清单：本阶段无 CSS 补丁；代码修复 `0a0d890` / `bc28ebc` 等见验收记录 §3。遗留观察：取消置顶不关闭浮窗（既有）、岛内提醒与轮播文案叠加（与原型一致）、合成 1ms 点击会被 80ms 轮询漏检（既有）。
