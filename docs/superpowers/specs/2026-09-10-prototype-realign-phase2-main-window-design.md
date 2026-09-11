# 原型对齐 · 阶段二「主窗口」设计

**日期**：2026-09-10
**状态**：已与用户逐节确认，待用户复核全文
**唯一参考原型**：`docs/index.html`（`#mainWindow` 段）+ `docs/styles.css` + `docs/app.js`
**前置**：阶段一「基础层」已完成并实机验收（`2026-09-10-prototype-realign-phase1-foundation-design.md`）。生成层样式已含本阶段全部原型选择器（`.note-arch-bar .clip-arch .clip-from .launcher-page-hero .island-preview .heat-* .trend-* .day-* .todo-date-chip mark .shake-tip .side-item[data-view=launcher|island]` 等），本阶段**不改生成层**。

---

## 1. 范围

阶段二 = 主窗口内的一切：

| 部分 | 内容 |
|---|---|
| 壳 | 标题栏、入场动效、视图切换、事件响应 |
| 侧边栏 | 五页签（笔记 / 粘贴板 / 待办 / 启动台 / 灵动岛）+ 计数徽章 + 当月迷你热力图 + 底部 ⚙️ / 📊 + 折叠 / 拖宽 |
| 视图 | 笔记页、粘贴板页、待办页、日期详情页、统计页、启动台页、灵动岛页、偏好设置页 |

**不在本阶段**：呼出面板（阶段三）；标签管理 / 待办编辑 / 粘贴板编辑弹窗、优先级 / 重复菜单、删除二次确认浮层 `#cardConfirm`、toast、置顶浮窗、提醒卡片、托盘菜单（阶段四）；思维导图窗口（阶段五）。这些在主窗口内被调用时沿用现有实现。

## 2. 决策记录（本阶段新增，均已确认）

| # | 问题 | 采用 | 理由 |
|---|---|---|---|
| D10 | 原型控件对应的后端缺失能力（启动台检索笔记 / 待办 / `=` 计算器 / 浏览器历史与保留天数；灵动岛播放范围 / 悬停穿透 / 流光边框 / 全屏自动隐藏；粘贴板来源应用） | 只做骨架与已有能力，缺失控件**不渲染**，留阶段四随启动台 / 灵动岛窗口一起补 | 页面上不出现无效控件；能力本身属于其他窗口 |
| D11 | 统计页趋势图 | 改为原型内联 SVG 折线，卸载 ECharts | 原型为准；ECharts 只此一处，卸载后主窗口包体积约减半 |
| D12 | 做法 | 逐视图就地对齐，抽共享件 | 可分批提交、分批验收 |
| D13 | 笔记 ✏️ 编辑入口 | 暂留主窗口弹窗，阶段三面板对齐后再切到「回显到面板 + Zen」 | 回显依赖面板的编辑态，属阶段三 |
| D14 | 提醒徽章 | 保留现状「⏰ 前 15 分钟 🔔」，不按原型的绝对时间 | 2026-09-03 已拍定的相对偏移 + 邮箱方案，产品决策优先于原型 |

## 3. 与原型的差异处理表

| # | 项 | 原型 | 现状 | 阶段二 |
|---|---|---|---|---|
| 1 | 卡片操作图标 | 内联 SVG（close / pin / edit / paste / link） | emoji | 改为原型 SVG，新建 `Icon.vue` |
| 2 | 笔记页工具栏 | 搜索框 + 「🧠 思维导图」按钮（`.note-arch-bar`） | 多一个类型筛选下拉；卡片多「📝 笔记 / 🧠 导图」徽章 | 删除下拉与徽章；导图卡片正文前显示 `<span class="clip-type mindmap">🧠 思维导图</span>` |
| 3 | 笔记搜索范围 | 正文 + 标签 + 导图全部节点文本 | 正文 + 标签 | 补导图节点文本 |
| 4 | 导图笔记正文 | 根节点文字 | 固定文案 | 显示根节点文字（`mindmap_data` 解析失败时显示「未命名导图」） |
| 5 | 卡片标签 ✕ | 悬浮显示；首次点击进入抖动确认，再次点击删除，3 秒无操作退出；抖动时展开全部标签并显示「再次点击 ✕ 确认删除」 | 卡片上不可删 | 补齐；真实写库删除 |
| 6 | 笔记 ✏️ | 回显到面板 | 弹窗 | 暂留弹窗（D13） |
| 7 | 粘贴板卡片 | 主窗口 `.archive-item.clip-arch`：正文两行截断、类型徽章、时间、右下操作组 | 复用面板 `.clip-item` | 新建 `ClipArchiveCard.vue`；面板卡片留阶段三 |
| 8 | 待办卡片结构 | `.todo-foot` 与 `.todo-head` 平级、撑满整行 | 底栏嵌在正文列内 | 按原型调整；主窗口列表加 `todo-arch-list`（解除 `.todo-list` 的 380px 限高） |
| 9 | 完成时间徽章 | 「📅 今天 HH:mm」/「📅 M/D HH:mm」 | 多「明天 / 昨天」 | 按原型（`formatDueLabel` 去掉两个分支，面板同用） |
| 10 | 提醒徽章 | 「⏰ 日期 时间」 | 相对偏移 + 渠道 | 保留现状（D14） |
| 11 | 待办搜索 | 命中文字 `<mark>` 高亮；只命中子任务时父级强制展开、命中项 `search-hit` 虚线框 | 无 | 补齐 |
| 12 | ＋子任务 | 已完成父级也可加（子任务 <5 时显示） | 仅未完成父级 | 按原型（后端已支持：`create_child_todo` 会重开父级） |
| 13 | 日期文案 | 待办页日期条 `YYYY-MM-DD`；日期详情标题 `YYYY-MM-DD` + ` · 今天` | 「2026年9月10日 周四（今天）」 | 按原型 |
| 14 | 「今天」按钮 | 常显 | 非今天才显示 | 按原型 |
| 15 | 侧栏计数 | 笔记总数 / 粘贴板总数 / 待办（含子任务与已完成） | 待办只数未完成顶级 | 按原型（笔记仍排除草稿——草稿不是列表里的卡片） |
| 16 | 迷你热力图 | 周一对齐；档位 `<3 / <6 / <10 / ≥10`；悬浮明细；标题「N月活跃（悬浮明细 · 点击查当日）」；选中日 `selected` 描边 | 周日对齐、无悬浮、标题「N 月活跃度」 | 按原型 |
| 17 | 统计热力图 | 182 天且起点对齐到周一；档位 `<5 / <10 / <18 / ≥18`；星期标签一 / 四 / 日；月份标签同列去重且间距 ≥3 列；图例含「存在逾期」；明细浮层在格子上方居中，越界翻到下方 | 周日对齐、明细跟随鼠标、无图例 | 按原型 |
| 18 | 趋势图 | 内联 SVG：620×190、内边距 l40 r12 t24 b28、y 轴 4 条网格线（0 / ⅓ / ⅔ / max，max 向上取整到 10）、三条折线 + 圆点 `<title>` 悬浮、图例右对齐 | ECharts | 按原型（D11） |
| 19 | 主窗口入场 | scale .94 → 1 + 淡入 220ms（`power2.out`） | 无 | animejs `popIn` 预设，时长 `--dur-base`；若 Mica 底衬露边则退化为纯淡入（实机定） |
| 20 | 键盘可达 | 页签 `role="tab" tabindex="0" aria-selected`；筛选 chip `role="button" aria-pressed`；Enter / Space 触发 | 无 | 补齐（ui-ux-pro-max：keyboard-nav / nav-state-active） |
| 21 | 设置页行序 | 失焦收起 → 灵动岛开关 → 保留天数 → 开机自启 → 全局快捷键 → 备注样式 → 主题 | 顺序不同；含灵动岛 / 启动器 / 面板插件 / 邮件分节 | 原型行按原型顺序；项目扩展项（面板唤出位置、窗口毛玻璃、玻璃质感、数据目录、面板插件、邮件提醒）排在主题之后；灵动岛设置整体搬到灵动岛页、启动器设置整体搬到启动台页 |
| 22 | 启动台页 | 标题 + 说明 + 内嵌试用区 + 启动台设置（快捷键、检索范围 ×5、历史保留天数、已记录历史、呼出浮窗按钮） | 无此页 | 标题 + 说明 + 内嵌试用区（真实搜索）+ 快捷键 + 现有启动器设置（全盘索引开关、排除目录、索引状态与重建、扫描根目录）+ 呼出浮窗按钮；检索范围与历史相关控件不渲染（D10） |
| 23 | 灵动岛页 | 标题 + 说明 + 预览 + 状态行 + 灵动岛设置（启用、停留时间、播放范围、透明度、点击穿透、悬停穿透、流光、全屏隐藏） | 无此页 | 标题 + 说明 + 预览 + 状态行 + 启用 / 轮播间隔 / 透明度 / 点击穿透 / 宽高 / 插件；播放范围、悬停穿透、流光、全屏隐藏不渲染（D10） |
| 24 | 侧栏页签 | 5 个 | 3 个 | 加启动台、灵动岛（无计数，`--tint` 回退强调色由生成层规则提供） |

## 4. 组件与文件

```
src/
├─ components/
│   ├─ base/Icon.vue                    新：原型五枚 SVG（close / pin / edit / paste / link），11px，currentColor
│   ├─ card/NoteCard.vue                改：原型结构；标签 ✕ 抖动确认；导图徽章与根节点正文
│   ├─ card/ClipArchiveCard.vue         新：主窗口粘贴板卡片（.archive-item.clip-arch）
│   ├─ card/TodoCard.vue                改：todo-foot 与 todo-head 平级；query 高亮；已完成父级 ＋子任务
│   ├─ card/TodoTree.vue                改：query / archive 属性；搜索命中强制展开与 search-hit
│   ├─ stats/HeatTip.vue                新：热力图悬浮明细（fixed，格子上方居中，越界翻下）
│   ├─ stats/MiniHeatmap.vue            新：侧栏当月迷你热力图（从 Sidebar 抽出）
│   └─ stats/TrendChart.vue             新：原型 SVG 折线
├─ utils/
│   ├─ heatmap.ts                       新：纯函数——周一对齐网格、档位、月份标签、迷你 / 统计两套阈值
│   ├─ search.ts                        新：highlight(text, query) → 分段；mindmapAllText(json)
│   ├─ todo.ts                          改：searchTodos(todos, query) → { roots, forceExpand, hitIds }
│   └─ datetime.ts                      改：formatDueLabel 去掉昨天 / 明天；新增 formatDateKey(key, withToday)
├─ constants/navigation.ts              改：五页签常量（key / icon / label / countKey?），Sidebar 接管使用
├─ typings/domain.ts                    改：View 扩为 8 个值
├─ motion/presets.ts                    改：新增 popIn(el)（scale .94→1 + 淡入，--dur-base，--ease-out）
├─ windows/Main/
│   ├─ MainApp.vue                      改：8 视图切换；入场 popIn；订阅 main-shown
│   ├─ Sidebar.vue                      改：五页签、计数口径、MiniHeatmap、role=tab 键盘
│   ├─ NotesView.vue                    改：note-arch-bar；搜索含导图文本；标签直删
│   ├─ ClipsView.vue                    改：ClipArchiveCard；容器 <div>
│   ├─ TodosView.vue                    改：日期条按原型；query 下发；todo-arch-list
│   ├─ DayView.vue                      改：标题、卡片细节、编辑入口
│   ├─ StatsView.vue                    改：heatmap.ts + HeatTip + TrendChart
│   ├─ LauncherPageView.vue             新
│   ├─ IslandPageView.vue               新
│   └─ SettingsView.vue                 改：行序；灵动岛 / 启动器分节迁出
├─ styles/extensions.css                改：删除 §5 的 .archive-page > ul 桥接
└─ service/events.ts                    改：mainShown
src-tauri/src/
├─ events.rs                            改：MAIN_SHOWN
├─ app/windows.rs                       改：show_main 末尾 emit MAIN_SHOWN
├─ ipc.rs + main.rs                     改：注册 launcher_show 命令（函数已存在）
```

### 4.1 共享件契约

- `Icon.vue`：`props.name: 'close' | 'pin' | 'edit' | 'paste' | 'link'`；`aria-hidden="true"`，按钮的可读名由外层 `title` 提供。
- `utils/heatmap.ts`
  - `buildMonthGrid(activity, year, month, selectedKey)` → `{ cells: (MonthCell | null)[] }`，周一对齐（`lead = (firstDay + 6) % 7`），null 为占位；`level(total, MINI_THRESHOLDS)`。
  - `buildRangeGrid(activity, days = 182, today)` → `{ cells, months: {label, column}[] }`，起点 = 今天 − 181 天再回退到周一；月份标签：每列（周）首格月份变化且与上一标签间距 ≥ 3 列时记录。
  - `alphaOf(level)`：`[0, .18, .38, .6, .88]`；`MINI_THRESHOLDS = [3, 6, 10]`、`STATS_THRESHOLDS = [5, 10, 18]`。
- `HeatTip.vue`：`props.day: ActivityDay`、`props.anchor: DOMRect`；自身测量宽高后定位（top = anchor.top − h − 8，<8 则 anchor.bottom + 8；left 居中并夹在 8 ~ innerWidth − w − 8）；入场用 animejs 淡入 + 4px 上移（`--dur-fast`）。
- `TrendChart.vue`：`props.months: MonthTrend[]`；`themeVar` 读 `--trend-note/--trend-clip/--trend-todo/--text-dim/--wsa`；监听 `settings.theme` 变化重算颜色；`role="img" aria-label="月度趋势折线图"`。
- `utils/search.ts`：`highlight(text, query): { text: string; hit: boolean }[]`（忽略大小写，只标首个命中）；`mindmapAllText(json: string | null): string`（兼容 `{root}` 与裸 `{data, children}` 两种历史格式，递归 `data.text` 去富文本标签后以空格拼接）。
- `utils/todo.ts::searchTodos(todos, query)` → `{ nodes: TodoNode[]; forceExpand: Set<string>; hitIds: Set<string> }`：命中父级 → 整棵树；只命中子任务 → 父级进 `forceExpand`，首个命中子任务进 `hitIds`；排序：日期降序 → 优先级。

### 4.2 各视图要点

**MainApp**：`view` 类型 `View | 'day'`；`navigate()` 切换视图时各视图组件卸载，其 `useConfirmDelete` 确认态随之复位（与原型 `clearPendingDelete` 等价，无需额外总线）；HeatTip 由 Sidebar 持有，切换时关闭；`onMounted` 与 `main-shown` 事件各触发一次 `popIn(root)`。

**Sidebar**：`NAV` 改为从 `constants/navigation.ts` 导入；页签为 `<div role="tab" tabindex="0" :aria-selected>`，`@keydown.enter/.space.prevent` 触发；计数 `notes = 非草稿总数`、`clips = 总数`、`todos = 全部条目数（含子任务与已完成）`；迷你热力图 `<MiniHeatmap :activity :selected-date @pick @hover>`，HeatTip 由 Sidebar 承载（`Teleport to="body"`）。

**NotesView**：`.note-arch-bar`（搜索框 + `btn tiny` 🧠 思维导图）；`visible` 过滤：正文 / 标签 / `mindmapAllText`；置顶优先 → 归档时间倒序；标签 ✕：`useShakeConfirm` 按 `note.id:tag` 键；确认后 `api.notes.save({...note, tags: without})`；`.archive-item.shaking` 时 `TagList` 强制展开并渲染 `.shake-tip`。

**NoteCard**：结构 `card-close(Icon close) → .a-text(导图徽章 + 正文 html) → .a-meta(📌? 时间 · TagList · .a-ops(pin / edit))`；导图笔记正文 = `mindmapRootText(note.mindmap_data) || '未命名导图'`。

**ClipArchiveCard**：`.archive-item.clip-arch(.pinned)`：`card-close` → `.a-text`（两行截断由原型 CSS 负责）→ `.a-meta`：`ClipTypeBadge` · 📌? 时间 · `.a-ops`（paste / pin / link 仅 `content_type === 'link'` / edit 仅非 image）。事件与现 `ClipCard` 相同：`paste pin edit open-link ask-delete confirm-delete cancel-delete`。

**TodosView / TodoTree / TodoCard**：日期条 `‹ YYYY-MM-DD › 今天(常显) 提示 搜索框 ＋`；非搜索态口径不变；搜索态改用 `searchTodos`；`TodoTree` 新增 `query`、`forceExpand`、`hitIds`、`archive`（加 `todo-arch-list` 类）属性；`TodoCard` 文本经 `highlight` 渲染 `<mark>`；`＋子任务` 显示条件 `depth === 0 && childCount < 5`。

**DayView**：标题 `${dateKey}${isToday ? ' · 今天' : ''}`；卡片按原型顺序渲染：时间 · 类型徽章 · 正文行（待办优先级徽章 / 粘贴板类型徽章 / 导图徽章 / 正文 d-clamp3|4 / 逾期）· 元信息行（提醒 `⏰ 前 N 分钟`、重复）· 标签 · 备注 · `子任务 · 属于「父级」`（父级文本从 `todos` 里按 `parent_id` 查）· `.day-ops`（edit / del）；编辑：笔记→`NoteEditModal` 或 `mindmapOpen`，粘贴板→`ClipEditorModal`，待办→`TodoEditorModal`（已完成拦截 toast）。

**StatsView**：`buildRangeGrid` + `HeatTip` + `TrendChart`；图例 `少 ▢▢▢▢ 多 ▢ 存在逾期`。

**LauncherPageView**：`useLauncherSearch()` 组合式函数（去抖 30ms、↑↓ 循环、Enter 执行、点击执行、`mouseenter` 选中）复用 `api.launcher.search/launch`；结果项 `.launcher-item(.active) > .li-ico .li-name(small=path) .li-cat(kind 中文)`；页脚 `↑↓ 导航 · ↵ 执行 · 点击直接执行 · 共 N 项`；空结果 `.launcher-empty`；设置分节把 `SettingsView` 现有启动器行整体搬来（含 `launcherStatus` 轮询与根目录编辑逻辑，抽成 `useLauncherSettings()` 供两处复用后再删除设置页那份）。

**IslandPageView**：预览 `.island-preview > .di-item.island-preview-item`：`.di-dot.{high|medium|low|idle}` + `.di-text` + `.di-time(.ovd)`（用 `pickTodayTodos` 第一条；无待办时 `.di-dot.idle` + `.di-text.dim` 「今日待办已全部完成 🎉」）；状态行 `当前播放 N 条待办 · 每条停留 S 秒 · 灵动岛已停用? · 点击穿透?`；设置行从 `SettingsView` 整体搬来。

**SettingsView**：按 §3-21 重排；`ISLAND_LIMITS`、`patchIslandNumber`、启动器相关逻辑随分节迁出；主题下拉与阶段一一致。

### 4.3 后端

- `events.rs`：`pub const MAIN_SHOWN: &str = "inkling://main-shown";`
- `windows::show_main` 末尾 `let _ = app.emit(events::MAIN_SHOWN, ());`
- `ipc.rs`：`pub fn launcher_show(app) -> Result<(), String> { windows::launcher_show(&app) }`，`main.rs` 注册；`tauri.ts` 增加 `launcher.show()`。

## 5. 测试

### 5.1 纯函数单元测试（`node --test`，TS 直接运行）
- 文件与被测模块同目录：`src/utils/heatmap.test.ts`、`search.test.ts`、`todo.test.ts`、`datetime.test.ts`。
- 工具模块之间用相对导入；`todo.ts` 现有 `import type` 自 `@/typings/domain` 为类型导入，Node 剥离后无运行时影响。
- `package.json`：`"test:unit": "node --test \"src/**/*.test.ts\""`，`"test": "pnpm test:scripts && pnpm test:unit"`。
- 用例：
  - heatmap：周一对齐（2026-09-01 是周二 → lead 1）；迷你 / 统计两套档位边界（2 / 3、4 / 5、9 / 10、17 / 18）；182 天起点回退到周一；月份标签去重与最小间距；逾期标记。
  - search：忽略大小写只标首个；无命中；`mindmapAllText` 对 `{root}`、裸 root、非法 JSON、空串。
  - todo：`searchTodos` 命中父级 / 只命中子任务 / 无命中 / 排序。
  - datetime：`formatDueLabel` 今天与非今天、跨年。

### 5.2 后端
`cargo check` + 既有 74 项测试全绿；新增事件与命令无数据逻辑，不加用例。

## 6. 实机验收（打字机 + 深色）

1. 主窗口从托盘 / 快捷键打开 5 次：入场缩放淡入、无 Mica 露边；露边则退化纯淡入并记入结论。
2. 侧栏：五页签切换、Tab + Enter/Space、计数口径、折叠 / 拖宽；迷你热力图悬浮明细定位、点击进详情、选中描边。
3. 笔记页：无类型筛选；导图卡片根节点文字 + 🧠 徽章；搜索命中导图节点；标签 ✕ 抖动确认直删、3 秒超时。
4. 粘贴板页：两行截断、类型徽章、link 外链、编辑仅文本类、置顶金色、无项目符号。
5. 待办页：底栏撑满整行、`YYYY-MM-DD`、常显「今天」、搜索高亮与子任务强制展开、已完成父级 ＋子任务、列表不被限高裁切。
6. 日期详情：徽章 / 逾期 / 提醒重复行 / 备注 / 子任务归属 / 编辑入口。
7. 统计页：周一对齐、图例、明细在格子上方；趋势 SVG 随主题换色、圆点悬浮；记录卸载 ECharts 后 `main-*.js` 体积。
8. 启动台页：内嵌搜索、↑↓ / Enter、设置行齐全、「呼出浮窗启动台」生效；设置页无启动器分节。
9. 灵动岛页：预览与胶囊同源、状态行、设置即时生效；设置页无灵动岛分节。
10. 设置页行序；主题下拉。
11. 面板待办页不破损（同一 TodoCard）。

## 7. 提交批次（中文）

① `feat(main): 共享件与纯函数（Icon / heatmap / search / TrendChart / HeatTip）+ 单测，卸载 echarts`
② `feat(main): 主窗口壳与侧边栏五页签、入场动效、main-shown 事件`
③ `feat(main): 笔记页对齐原型（工具栏 / 导图卡片 / 标签直删 / 导图文本搜索）`
④ `feat(main): 粘贴板页归档卡片对齐原型`
⑤ `feat(main): 待办页对齐原型（卡片结构 / 日期条 / 搜索高亮 / 子任务）`
⑥ `feat(main): 日期详情页对齐原型`
⑦ `feat(main): 统计页热力图与 SVG 趋势图对齐原型`
⑧ `feat(main): 启动台页（内嵌搜索 + 启动器设置迁入）与 launcher_show 命令`
⑨ `feat(main): 灵动岛页（预览 + 灵动岛设置迁入）`
⑩ `feat(main): 偏好设置页行序对齐原型`
⑪ `docs(design): 阶段二实机验收记录与结论回填`

## 8. 风险

| 风险 | 应对 |
|---|---|
| 主窗口入场 scale 与 DWM Mica 底衬露边 | 实机验收第 1 项决定；退化为纯淡入 |
| `TodoCard` 结构改动波及面板待办页 | 验收第 11 项只保证不破损；面板对齐在阶段三 |
| 标签直删为真实写库 | 抖动确认 + 3 秒超时，与原型一致 |
| `formatDueLabel` 改动影响面板与提醒卡片文案 | 全局一致即原型口径，接受 |
| Node 直接运行 TS 测试对 `import type` 之外的别名不支持 | 工具模块只用相对导入；测试不 import Vue 组件 |

## 9. 实机结论（2026-09-11 回填，详见 `../plans/2026-09-10-prototype-realign-phase2-acceptance.md`）

- 主窗口入场 scale .94 是否保留：**保留**。逐帧连拍 +169ms 为缩放半透明态、+219ms 到位，缩放边带透出的是窗口后方内容，无 Mica / Acrylic 底衬露边（150% 缩放、毛玻璃开）。
- 卸载 ECharts 前后 `main-*.js` 体积：556 KB → 59.2 KB（gzip 20.5 KB）。
- 补丁清单：`extensions.css` §9 `#launcherPageInput`（原型只定义了浮窗 `#launcherInput`，原型遗漏，原型补上后删）；`LauncherPageView.vue` 说明文案改脚本内拼接（模板换行在中文间引入空格）；`IslandPageView.vue` 「点击穿透」说明对齐实际行为；`ipc::launcher_show` 改 async（同步命令建窗死锁，与 `pin_create` 同款）。
- 验收覆盖：打字机主题跑完大部分条目；**深色主题一轮与若干需造数据的条目未验证**，清单见验收记录 §6。
- 既有缺陷记入阶段四：`settings_save` 同步建窗（灵动岛关→开死锁）、启动期间偶发原生崩溃（全盘索引扫描期间，三次）、启动台索引计数未含全盘文件。
