# 灵动岛（Dynamic Island）

主屏顶部居中、悬浮于桌面之上的胶囊窗口，轮播展示「当日待办」，悬停看详情，点击唤出面板到待办页。

## 用法

- **默认展示**：当日**未完成的顶级待办**（不含子任务），当天到期或此前逾期的都算，按完成时间升序。多条时循环轮播，右侧显示 `当前/总数`。
- **悬停**：停在胶囊上暂停轮播并展开详情——标题全文、完成时间、优先级、标签、备注（前 120 字）、逾期标记。
- **左键点击**：唤出呼出面板并切到「待办」页。
- **空态**：没有当日待办时显示「今天没有待办 · 点此新建」。

## 设置（偏好设置 → 灵动岛）

| 项 | 默认 | 范围 | 说明 |
|---|---|---|---|
| 显示灵动岛 | 开 | | 关闭即销毁窗口 |
| 宽度 | 360 | 200–800 | 逻辑像素 |
| 高度 | 36 | 28–72 | 逻辑像素，胶囊圆角 = 高/2 |
| 背景不透明度 | 0.85 | 0.3–1.0 | 只作用于胶囊背景，文字始终清晰 |
| 鼠标穿透（零干扰） | 关 | | 开启后鼠标事件全部穿透到下层，此时悬停/点击由后端光标探测 |
| 轮播间隔 | 4s | 2–30s | |
| 插件 | 当日待办 | | 见下「可插拔」 |

所有设置**改动即时生效**（窗口尺寸/位置/穿透在 Rust 侧重新应用，透明度/轮播/插件由前端热更新）。越界数值在前端与 Rust 两侧都会钳到范围内。

## 位置与多屏

灵动岛只在**主显示器**顶部居中，落在感应区（唤出面板用）正下方 4px，两者矩形不重叠——悬停灵动岛不会误触发感应区的「3 秒唤出」计时。多屏时其他屏不显示灵动岛（但每块屏仍有顶部感应区）。

## 可插拔设计

灵动岛内容由**编译期插件注册表** `src/island-plugins/index.ts` 提供，与呼出面板的插件同一套范式：

```ts
export interface IslandPlugin {
  id: string
  label: string
  useItems: () => Ref<IslandItem[]>   // 响应式条目列表
  panelPage?: string                  // 点击默认跳转的面板页
  emptyText?: string
}
```

- 启用与顺序由 `Settings.island_plugins`（逗号分隔的有序 id）决定，多个插件的条目拼接后统一轮播。
- 新增一种展示内容：实现 `IslandPlugin`、加进 `builtinIslandPlugins`，无需改根组件。
- 本期内置 `today-todos` 一个插件。
- 不支持运行时加载外部脚本（CSP `script-src 'self'`，与呼出面板插件同一约束）。

## 实现要点

- 窗口 `island`：透明、置顶、不进任务栏、不抢焦点；物理像素定位（多屏混合 DPI 下不落错屏）。
- 悬停/点击统一由 `hotzone_watcher` 的 80ms 光标轮询探测：进入/离开翻转发 `island-hover`；光标在矩形内且左键**按下边沿**触发点击（`GetAsyncKeyState(VK_LBUTTON)`）。穿透与非穿透模式因此行为一致。
- 点击唤出面板用 `panel_show_page("todo")`：目标页先存入 `AppState`，面板显示后再取（隐藏的 WebView 收不到事件）。

## 代码位置

- Rust：`src-tauri/src/app/windows.rs`（`create_island` / `island_apply` / `island_expand` / `panel_show_page`）、`src-tauri/src/services/hotzone_watcher.rs`（悬停/点击探测）、`src-tauri/src/app/state.rs`（`island_rect` / `pending_panel_page`）。
- 前端：`src/windows/Island/IslandApp.vue`、`src/island-plugins/`、`src/styles/island.css`。

## 已知限制

- 仅 Windows 实现左键探测（`GetAsyncKeyState`）；其他平台穿透模式下点击不可用。
- 只在主屏显示；将来若要「每屏一个」，`island` 设计里预留了扩展点。
