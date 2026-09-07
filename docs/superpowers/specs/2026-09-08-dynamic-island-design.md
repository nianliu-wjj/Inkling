# 灵动岛（Dynamic Island）设计

**日期**：2026-09-08
**状态**：待评审（决策已按推荐方案落定并逐条记录，用户可逐条否决）
**对应需求**：用户 2026-09-07 提出：

> 在屏幕顶部的中间位置悬浮于桌面上的一个胶囊样式的灵动岛样式，支持自定义长宽高（但是需要限制最小/大值），
> 支持配置窗口半透明化、鼠标穿透（零干扰界面），默认展示内容是当日待办实现中的待办项（不展示子任务），
> 并且循环滚动展示当天的待办项，只有鼠标悬浮在灵动岛上时，展示该待办项的详细信息，
> 鼠标左键单击灵动岛时，唤出面板且展示在待办项窗口。该灵动岛需要可插拔设计，方便通过插件方式引入更多功能。

## 关于本文档的决策方式

自主运行会话，沿用 09-03 / 09-06 spec 的先例：决策点以「问题 / 备选 / 采用 / 理由」记录，不逐个打断用户。

## 现状盘点

| 已有能力 | 位置 | 与灵动岛的关系 |
|---|---|---|
| 感应区窗口（每屏一个，透明穿透，顶部居中 240×80） | `app/windows.rs` `create_hotzone_window`，`services/hotzone_watcher.rs` | 灵动岛就是「看得见的感应区」；悬停唤出面板的逻辑可直接复用 |
| 全局光标轮询（80ms，物理像素矩形判定） | `hotzone_watcher.rs` | 穿透模式下悬停/点击都靠它探测 |
| 面板插件注册表（编译期、`Settings.panel_plugins` 有序列表） | `src/panel-plugins/index.ts` | 灵动岛插件采用同一套注册表范式 |
| 今日待办口径（顶级、非子任务、当天 + 逾期） | `windows/Panel/TodoPage.vue` `visible` 计算 | 默认内容插件直接复用 `useTodos` + 同一筛选 |
| 设置模型 + v4 迁移 | `domain/models.rs` `Settings`、`data/settings.rs`、`data/mod.rs` | 新增灵动岛字段走同一条路 |

## 决策记录

### D1：灵动岛与感应区的关系

**备选**：A. 独立窗口，与感应区并存；B. **灵动岛替代主屏感应区的「可见部分」，共用同一套悬停探测**；C. 灵动岛完全取代感应区
**采用**：B。主屏（primary monitor）的感应区窗口保留（它负责「悬停 3 秒唤出」的计时动画），灵动岛作为**独立窗口** `island` 叠在同一位置之下方 4px 处；其他屏的感应区不变。
**理由**：感应区的物理矩形记账、多屏对账、穿透与轮询都已稳定，不必重写。灵动岛只需增加「展示内容 + 悬停详情 + 点击」三件事。放在感应区下方一点而不是完全重叠，是为了悬停灵动岛时不误触发 3 秒唤出计时（两者矩形不重叠）。

### D2：只在主屏显示

**采用**：灵动岛只在**主显示器**顶部居中显示；多屏时其他屏不显示灵动岛（感应区仍每屏一个）。
**理由**：需求原文是「屏幕顶部的中间位置」单数；待办提醒也已约定只在主屏。将来若要每屏一个，`island_monitor` 设置项预留 `primary | all` 取值，本期只实现 `primary`。

### D3：尺寸与限值

**采用**：`island_width` 逻辑像素 **[200, 800]**，默认 360；`island_height` **[28, 72]**，默认 36；圆角 = 高度 / 2（胶囊）。悬停展开详情时高度临时增至 `max(island_height, 120)`，宽度不变；展开态窗口向下延伸，不改变顶部位置。设置页用数字输入 + 滑块，越界值在 Rust 侧 clamp（前端也 clamp，双保险）。
**理由**：200 以下放不下一条待办的标题 + 时间；800 以上在 1080p 上超过半屏；高度 28 是 12px 字号加内边距的下限，72 以上不再像胶囊。

### D4：半透明与穿透

**采用**：
- `island_opacity` **[0.3, 1.0]**，默认 0.85，作用于胶囊背景（CSS 变量 `--island-alpha`），文字不透明。
- `island_click_through`（默认 **false**）：为 true 时窗口 `set_ignore_cursor_events(true)`——鼠标事件全部穿透到下层桌面/应用（零干扰）。此时**悬停与点击改由 Rust 轮询探测**（见 D5），前端不再收 DOM 鼠标事件。
**理由**：穿透是窗口级属性，只能在 Rust 设置；但需求同时要求悬停详情与点击唤出，所以穿透模式必须有另一条事件来源。

### D5：穿透模式下如何探测悬停与点击

**备选**：A. 全局鼠标钩子 `SetWindowsHookEx(WH_MOUSE_LL)`（需独立消息循环线程）；B. **复用 80ms 光标轮询 + `GetAsyncKeyState(VK_LBUTTON)` 轮询左键状态**；C. 穿透模式下放弃点击功能
**采用**：B。`hotzone_watcher` 每轮同时判定光标是否在灵动岛矩形内（矩形由 Rust 建窗时记账，同感应区）：
- 进入/离开翻转 → `emit_to("island", ISLAND_HOVER, bool)`；
- 在矩形内且左键从「未按」变为「按下」（边沿检测）→ `emit_to("island", ISLAND_CLICK)` 并由 Rust 直接执行 `panel_show` + 广播 `PANEL_NAVIGATE("todo")`。
非穿透模式下前端用 DOM `mouseenter/mouseleave/click` 走同样的 bus 事件名，两条来源汇合到同一处理函数。
**理由**：低级钩子要单独线程跑消息循环、还要考虑钩子被系统摘除的恢复，代价高；80ms 轮询在既有线程里加两行判断即可，按键按下通常持续 >100ms，边沿检测足够可靠。`windows-sys` 已是依赖，只需加 `Win32_UI_Input_KeyboardAndMouse` feature。

### D6：默认内容与循环滚动

**采用**：默认插件 `today-todos`：
- 数据：`useTodos()` 全量 → 筛选 `!parent_id && status !== 'done' && (dateKeyOf(due_at) === today || isOverdue)`，按 `due_at` 升序；**不含子任务**。
- 展示：一次一条，「优先级色点 + 标题（单行省略）+ 完成时间 HH:mm」；每 **4 秒**切到下一条（可配 `island_cycle_seconds` [2, 30]），切换用向上滚动 240ms 动效；悬停时**暂停轮播**。
- 空态：「今天没有待办 · 点此新建」。
- 详情（悬停）：标题全文、完成时间（含日期）、优先级、标签、备注前 120 字、逾期标记。
**理由**：与托盘提示、面板待办页同一口径；「进行中」在本项目数据模型里即 `status !== 'done'`。

### D7：点击行为

**采用**：左键单击 → `panel_show`（面板从设置的边弹出、落在光标所在屏）→ 面板收到 `PANEL_NAVIGATE` 事件切到 `todo` 插件页。空态点击同样打开待办页（面板待办页顶部就是新建输入）。
**理由**：需求原文；`PANEL_NAVIGATE` 新事件与主窗口的 `NAVIGATE` 分开，避免主窗口误切视图。

### D8：可插拔设计

**采用**：编译期注册表 `src/island-plugins/index.ts`，与 `panel-plugins` 同范式：

```ts
export interface IslandItem {
  id: string
  /** 胶囊单行主文案 */
  title: string
  /** 胶囊右侧短文案（时间 / 数量等） */
  meta?: string
  /** 左侧色点（优先级 / 状态色），CSS 颜色 */
  dot?: string
  /** 悬停详情组件；不提供则悬停只放大显示 title */
  detail?: Component
  /** 点击行为；不提供则走默认（唤出面板到该插件声明的 panelPage） */
  onClick?: () => void | Promise<void>
}
export interface IslandPlugin {
  id: string
  label: string
  /** 返回响应式条目列表；组合式函数，在灵动岛根组件 setup 内调用一次 */
  useItems: () => Ref<IslandItem[]>
  /** 点击默认跳转的面板插件 id */
  panelPage?: string
  /** 空态文案 */
  emptyText?: string
}
```
启用与顺序由 `Settings.island_plugins`（逗号分隔有序 id）决定，条目按插件顺序拼接后统一轮播。本期内置 `today-todos` 一个插件；注册表、设置页多选排序与轮播逻辑都按多插件实现，加第二个插件不必改根组件。
**理由**：与面板插件同一心智模型；不做运行时加载外部脚本（CSP `script-src 'self'`，理由见 `panel-plugins/index.ts` 头注释）。

### D9：设置项与迁移

`Settings` 新增（v5 迁移只是新键有默认值，settings 表是 KV，不需要 ALTER）：

| 键 | 类型 | 默认 | 范围 |
|---|---|---|---|
| `island_enabled` | bool | true | |
| `island_width` | i64 | 360 | 200–800 |
| `island_height` | i64 | 36 | 28–72 |
| `island_opacity` | f64 | 0.85 | 0.3–1.0 |
| `island_click_through` | bool | false | |
| `island_cycle_seconds` | i64 | 4 | 2–30 |
| `island_plugins` | String | "today-todos" | 逗号分隔 |

设置页新增「灵动岛」分区。改动即时生效：Rust 收到 `settings_save` 后 `island_apply(app)` 重设尺寸/位置/穿透/显隐；前端灵动岛窗口订阅 `settings-changed` 更新透明度、轮播间隔、插件列表。

### D10：窗口属性

`island` 窗口：`decorations(false)`、`transparent(true)`、`always_on_top(true)`、`skip_taskbar(true)`、`focused(false)`、`resizable(false)`、`shadow(false)`；位置：主屏工作区顶部居中、y = 感应区高度 + 4（逻辑）→ 感应区（0–80）在上、灵动岛（84–120）在下，二者不重叠；物理像素落位并记账矩形（同感应区）。悬停展开时 Rust `island_expand(true|false)` 改高度。DWM 圆角对透明窗口无效，圆角由 CSS 胶囊背景实现。

## 架构

```
Rust
  app/windows.rs      create_island / island_apply / island_expand / island_rect 记账（AppState.island_rect）
  services/hotzone_watcher.rs  每轮：island 悬停翻转 → ISLAND_HOVER；左键边沿 → ISLAND_CLICK + panel_show + PANEL_NAVIGATE
  ipc.rs              island_expand(expanded: bool)、settings_save 后调用 island_apply
  events.rs           ISLAND_HOVER / ISLAND_CLICK / PANEL_NAVIGATE
  domain/models.rs + data/settings.rs   7 个 island_* 字段
前端
  island.html + src/windows/island.ts + src/windows/Island/IslandApp.vue   胶囊 + 轮播 + 详情
  src/island-plugins/index.ts + today-todos.ts(+ TodoDetail.vue)
  src/windows/Panel/PanelApp.vue   订阅 PANEL_NAVIGATE 切页
  src/windows/Main/SettingsView.vue  「灵动岛」分区
  src/styles/island.css
```

数据流：`useTodos()` 在灵动岛窗口里订阅 `todos-changed` 自动刷新 → 插件 `useItems` 计算 → 根组件轮播索引 → 胶囊渲染。悬停：DOM 事件或 `ISLAND_HOVER` → `expanded=true` → `invoke('island_expand', true)` 拉高窗口 → 渲染详情；离开反之。

## 错误处理与日志

- 主屏找不到 → 不建灵动岛，日志 `[island] 未找到主显示器，跳过`。
- 建窗失败不影响其他窗口（`create_core_windows` 里 `if let Err` 记日志继续）。
- 设置值越界：Rust clamp 后回写日志「[island] width 900 超出范围，按 800 生效」。

## 测试

- Rust 单测：`island_geometry(work, width, height, hotzone_height)` 几何（1.0 / 1.5 缩放）、设置 clamp 函数。
- 前端：`vue-tsc`；实机：默认显示、轮播、悬停展开、点击唤出并切到待办页、穿透模式下的悬停/点击、设置项即时生效。

## 未决

无。
