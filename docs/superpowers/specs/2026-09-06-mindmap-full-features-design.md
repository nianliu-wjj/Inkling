# 思维导图全功能实现（对齐 simple-mind-map 参考端）设计

**日期**：2026-09-06
**状态**：待评审（决策已按推荐方案落定并逐条记录，用户可逐条否决）
**对应需求**：用户 2026-09-06 提出「思维导图的功能参考项目 `D:\参考项目\mind-map`，将思维导图的所有功能在本项目中实现」

## 关于本文档的决策方式

本次为自主运行会话，用户不在线逐题确认。沿用 2026-09-03 spec 的先例：
每个决策点以「问题 / 备选 / 采用 / 理由」四段记录，未逐个打断用户。
用户可据此逐条复核；若某条不认可，只需改该条，不影响其余设计。

## 现状盘点

| 方面 | 参考项目（`D:\参考项目\mind-map`） | 本项目（Inkling） |
|---|---|---|
| 库 | `simple-mind-map` 0.14.0-fix.3（源码在仓库内） | 已依赖同版本，`node_modules/simple-mind-map/src/plugins/*` 完整可引用 |
| 插件 | 20 个插件全部注册（`web/src/pages/Edit/components/Edit.vue`） | **零插件**：裸 `new MindMap({el,data,fit,enableFreeDrag,mousewheelAction})` |
| 编辑 UI | Vue 2 + element-ui + vuex + vue-i18n，`web/src/pages/Edit/components` 共 48 个组件、约 1.46 万行 | 只有一张画布，无工具栏、侧栏、右键菜单 |
| 数据 | `{ root, layout, theme: {template, config}, view }` 全量存 localStorage | `note.mindmap_data` 只存 `getData(false)` 的 root，**主题/结构/视图不持久化** |
| 窗口 | 浏览器单页 | 独立 Tauri 窗口 `mindmap-<id>`，顶部有 Inkling 操作条（标题 / 标签 / 保存 / 关闭） |
| 主题 | 库默认主题 + `simple-mind-map-plugin-themes`（light/dark 两组数十套） | 仅库默认主题 |
| UI 组件库 | element-ui | `naive-ui` 已在 `package.json` 依赖中但**从未使用** |

## 范围

### 做（按参考端功能分区逐项对齐）

| 分区 | 功能 |
|---|---|
| 工具栏 | 回退 / 前进 / 格式刷 / 同级节点 / 子节点 / 删除节点 / 图片 / 图标 / 超链接 / 备注 / 标签 / 概要 / 关联线 / 公式 / 附件 / 外框 / 导入 / 导出 |
| 右键菜单（节点） | 插入同级 / 子级 / 父节点 / 概要，上移 / 下移，展开 / 收起所有下级，删除 / 仅删除当前，复制 / 剪切 / 粘贴，移除超链接 / 备注 / 图片 / 自定义样式，导出该节点为图片，编号其子节点，添加 / 删除待办，链接到指定节点 / 修改 / 删除节点链接 |
| 右键菜单（画布） | 回到根节点，展开 / 收起所有，展开到第 N 级，一键整理布局，适应画布，禅模式，一键去除所有节点自定义样式，复制到剪贴板（SMM / JSON / Markdown / Txt / 图片） |
| 侧栏 | 节点样式（常态 / 选中态：文字、边框、背景、形状、线条、内边距、渐变、图片布局、标签样式）/ 基础样式（背景色与图片、连线、根节点连线起点、节点内外边距、图标大小、二级及以下节点、边框风格、关联线样式、外框内边距、彩虹线条、字体字号）/ 主题（分组列表 + 预览图 + 覆盖自定义样式确认）/ 结构（14 种布局缩略图）/ 大纲（树形编辑，拖拽排序）/ 设置（性能模式、自由拖拽、富文本、滚轮行为、缩放方向、新建节点行为、实时渲染、滚动条、手绘风格、动量、演示填空、水印全部选项、展开按钮常显、键入自动进入编辑、连线样式继承、拖拽导入、图片文本间隔、内容间隔）/ 快捷键表 / 图标与贴纸 / 公式 / 备注 |
| 画布层 | 导航工具栏（结构下拉、回到根节点、搜索、缩放、小地图开关、只读切换、演示、全屏查看 / 编辑、鼠标行为切换、快捷键表入口）、小地图、滚动条、搜索与替换、字数 / 节点数统计、富文本浮动工具栏、节点图标浮动栏、图片位置浮动栏、备注内容悬浮展示、图片预览、外框样式面板、标签样式面板、关联线样式面板、大纲全屏编辑、源码编辑（JSON 查看 / 格式化 / 复制 / 应用） |
| 导入导出 | 导入：`.smm` / `.json` / `.xmind`（多画布选择）/ `.md`（文件或粘贴文本）/ 拖文件到窗口；导出：SMM / JSON / PNG / SVG / PDF / Markdown / XMind / Txt（含文件名、是否含配置、透明背景、内边距、多页、底部文字、完整背景图等选项） |
| 演示 | 演示模式（含填空模式开关） |
| 其他 | 水印（含仅导出显示）、禅模式、暗色画布（由导图主题决定）、拖拽导入遮罩、剪贴板文本智能粘贴（按换行拆分节点确认） |

### 不做（及理由）

| 参考端功能 | 不做的理由 |
|---|---|
| AI（一键生成 / 续写 / 对话 / AI 配置） | 依赖火山方舟大模型 key 与「思绪思维导图」桌面客户端本地 3456 端口转发；Inkling 没有 AI 后端与凭据体系。若将来要做，应作为独立子项目单独设计 |
| 协同编辑（Cooperate） | 参考端自身已注释掉；需要 WebRTC 信令服务器 |
| 本地文件目录树 / 新建 / 打开 / 另存为 | 基于浏览器 File System Access API；Inkling 的导图是 SQLite 中的笔记，「打开」由导入覆盖、「另存为」由导出覆盖 |
| 下载客户端 / 官网 / 版本号菜单项、网页版试用提示、多语言切换 | 与 Inkling 无关；Inkling 只有中文 |
| FreeMind（.mm）与 Excel（.xlsx）导出 | 参考端 `Export.vue` 的 `downTypeList` 计算属性把这两项都过滤掉了，网页版实际不提供；0.14.0 库源码 `src/parse` 也无对应转换器 |

## 决策记录

### D1：怎么「实现所有功能」——改写参考端 UI，还是嵌入参考端整包？

**备选**
- A. 把参考端 `web` 整体作为 iframe / 独立 Vue 2 应用嵌进导图窗口
- B. **在 Vue 3 中逐组件重写参考端编辑 UI，复用同一个 `simple-mind-map` 库与全部插件**
- C. 放弃参考端 UI，只注册插件、靴带式自造精简 UI

**采用**：B
**理由**：A 要在一个 WebView 里跑两套 Vue（2 与 3）、两套组件库、vuex + i18n，且 iframe 与 Tauri IPC、主题令牌、标签保存都隔着一层，维护成本最高。C 做不到「所有功能」。B 工作量最大但可控：库与插件层原样复用（这是功能的 80%），UI 层只是把 element-ui/vuex/i18n 的胶水换成 naive-ui/组合式函数/中文常量，逻辑可逐组件对照移植。

### D2：UI 组件库

**备选**：naive-ui（已在依赖中）/ element-plus（新增依赖）/ 全部手写
**采用**：naive-ui，按需引入，仅在导图窗口使用
**理由**：颜色选择器、滑块、选择器、开关、标签页、气泡、抽屉一应俱全，与 element-ui 逐一对应，移植成本最低；tree-shaking 友好；不进面板窗口，不影响需求「1 秒原则」。

### D3：视觉风格

**备选**：照搬参考端白底 element 风格 + isDark 开关 / **沿用 Inkling 的令牌与玻璃质感**
**采用**：沿用 Inkling 令牌（`--text / --text-dim / --accent / --wsa / --menu-bg / .glass`），naive-ui 用 `darkTheme` + `themeOverrides` 把主色、边框、背景映射到这些令牌。参考端的 `isDark` 开关不做（Inkling 有 30 套配色主题 + 3 档玻璃质感统一管理）。画布背景由**导图主题**决定，与窗口 chrome 无关。
**理由**：与主窗口、面板视觉一致；避免在导图窗口出现第二套白色 UI。

### D4：文档数据持久化

**备选**：继续只存 root / **存 `getData(true)` 全量**
**采用**：`note.mindmap_data` 存 `mindMap.getData(true)` 的 JSON：`{ root, layout, theme: { template, config }, view }`。读取时兼容：解析结果含 `root` 字段 → `setFullData`；否则视为旧格式的 root → `setData`。
**理由**：主题、结构、视图位置是导图的一部分，不持久化等于每次打开都回到默认主题与逻辑结构图。

### D5：窗口级偏好（非文档数据）存哪

**备选**：加进 `Settings`（SQLite）/ **localStorage** / 每张导图各存一份
**采用**：`localStorage` 两个键，与参考端同形：
- `inkling.mindmap.localConfig`：编辑器行为偏好 `isZenMode / openNodeRichText / useLeftKeySelectionRightKeyDrag / isShowScrollbar / enableDragImport`。
- `inkling.mindmap.config`：「设置」侧栏里的库实例配置（`enableFreeDrag / mousewheelAction / mousewheelZoomActionReverse / createNewNodeBehavior / openRealtimeRenderOnNodeTextEdit / isUseHandDrawnLikeStyle / isUseMomentum / alwaysShowExpandBtn / enableAutoEnterTextEditWhenKeydown / enableInheritAncestorLineStyle / imgTextMargin / textContentMargin / openPerformance / demonstrateConfig / watermarkConfig`），建实例时展开进 options，改动时 `updateConfig` 即时生效。
两者跨导图共享。`getData(true)` 只含 `root / layout / theme / view`，**不含**这些配置，与参考端 `storeConfig` 分离存储一致。
**理由**：这些是「我这台机器上怎么用编辑器」的偏好，不是笔记内容；进 `Settings` 要改 Rust 模型、迁移与设置页，收益为零。

### D6：保存策略

**备选**：纯手动 ⌃S / 纯自动 / **手动 + 已落库导图自动保存**
**采用**：保留现有「⌃S 保存 + 未保存标记 + 关闭确认」；此外对**已有 id** 的导图，`data_change` / `view_data_change` 去抖 1.5s 后自动保存并清除未保存标记。新建导图仍需首次手动保存（否则每打开一次新建窗口就会静默产生一条空笔记）。
**理由**：参考端是随改随存；Inkling 已有的手动保存不能退化，但连续编辑几十分钟只靠手动保存风险太大。

### D7：导出落盘方式

**备选**：`<a download>`（参考端做法）/ **系统保存对话框 + Rust 写文件**
**采用**：库的 `mindMap.export(type, false, name, ...)` 拿到数据（dataURL / 字符串 / Blob），前端转 base64，调用新增 Rust 命令 `write_file_base64(path, base64)`；路径来自 `@tauri-apps/plugin-dialog` 的 `save()`，按格式设扩展名过滤。
**理由**：Tauri WebView 内 `<a download>` 行为不稳定且无法指定目录；Rust 写文件与既有 `export_items` 一致。

### D8：导入文件读取

**采用**：`<input type="file">` + `FileReader`（参考端同样做法），加窗口级拖放。不引入 `plugin-fs`。
**理由**：WebView2 内文件选择器可用，File 对象直接交给库的解析器（`xmind.parseXmindFile` / `markdown.transformMarkdownTo`），零改动。

### D9：静态资源

**采用**：
- 工具栏图标：复制参考端 `web/src/assets/icon-font/`（iconfont.css + woff2/woff/ttf，88KB）到 `src/assets/mindmap/icon-font/`，只保留 woff2 + css（其余格式删掉，现代 WebView2 只需 woff2）。
- 结构缩略图：复制 `assets/img/structures/*.jpg`（132KB）到 `src/assets/mindmap/structures/`。
- 主题预览图：来自 `simple-mind-map-plugin-themes/themeImgMap`（随包）。
- 贴纸：参考端 `config/icon.js`（560KB base64 PNG）与 `config/image.js`（2.1MB SVG 目录）。**只移植 icon.js 的贴纸组**，`image.js` 的 2.1MB SVG 贴纸不移植；两者都放到独立异步 chunk（`import()`），打开「图标/贴纸」侧栏时才加载。
- 图片加载失败占位 SVG：复制。
**理由**：参考项目 MIT 许可；2.1MB 的 SVG 贴纸对一个便签应用性价比过低，且不影响任何编辑能力（节点图标仍有库内置 + icon.js 两组）。

### D10：新增依赖

| 依赖 | 用途 |
|---|---|
| `simple-mind-map-plugin-themes` | 主题分组列表、预览图与注册 |

`naive-ui` 已有。`katex`、`quill`、`jszip`、`pdf-lib` 随 `simple-mind-map` 已装。不引入 `xlsx`（参考端网页版并不提供 Excel 导出，见「不做」表）。

### D11：目录与模块边界

```
src/windows/MindMap/
  MindMapApp.vue              # 窗口壳：Inkling 操作条 + 编辑器区域；保存/关闭/标签（现有）
  core/
    createMindMap.ts          # 注册插件、构造实例、转发事件到 bus、加载可选插件（RichText/Scrollbar）
    plugins.ts                # 全部插件 usePlugin 与主题包 init（等价 full.js）
    persistence.ts            # 全量数据 <-> 字符串；旧格式兼容；纯函数，有单元测试
    localConfig.ts            # 窗口级偏好读写（localStorage）；纯函数 + 组合式函数
    bus.ts                    # 极简事件总线（mitt 风格，自实现 40 行），替代 vuex + $bus
    store.ts                  # reactive 共享状态：activeSidebar / isReadonly / isZenMode / isOutlineEdit / 当前激活节点列表
    useMindMap.ts             # provide/inject：组件取 mindMap 实例与 store
  constants/
    lists.ts                  # 字体/字号/颜色/边框/线型/形状/背景/编号/渐变/对齐/结构分组（参考端 config/zh.js 移植）
    shortcuts.ts              # 快捷键表
    formulas.ts               # 常用公式
    rainbow.ts                # 彩虹线条配色
    stickers.ts               # 贴纸组（异步 chunk）
  chrome/                     # 常驻 UI
    Toolbar.vue  NodeButtons.vue  NavigatorToolbar.vue  SidebarTrigger.vue  Count.vue  Scale.vue
    Navigator.vue(小地图)  ScrollbarBars.vue  Fullscreen.vue  MouseAction.vue  Demonstrate.vue
  sidebars/
    SidebarShell.vue  NodeStyleSidebar.vue  BaseStyleSidebar.vue  ThemeSidebar.vue  StructureSidebar.vue
    OutlineSidebar.vue  SettingSidebar.vue  ShortcutSidebar.vue  IconSidebar.vue  FormulaSidebar.vue  NoteSidebar.vue
  popups/                     # 跟随节点/选区的浮动层
    ContextMenu.vue  RichTextToolbar.vue  NodeIconToolbar.vue  NodeImgPlacementToolbar.vue  NoteContentShow.vue
    NodeImgPreview.vue  OuterFramePanel.vue  TagStylePanel.vue  AssociativeLineStylePanel.vue  SearchBox.vue
  dialogs/
    ImportDialog.vue  ExportDialog.vue  NodeImageDialog.vue  NodeHyperlinkDialog.vue  NodeNoteDialog.vue
    NodeTagDialog.vue  OutlineEditDialog.vue  SourceCodeDialog.vue  NodeLinkDialog.vue
  widgets/
    ColorField.vue(色板 + 更多颜色)  NumberField.vue  SelectField.vue  SwitchField.vue  FieldRow.vue
src/assets/mindmap/           # icon-font / structures / 图片加载失败.svg
src/typings/simple-mind-map.d.ts   # 扩展：MindMap 主类常用方法、插件实例属性、`simple-mind-map/src/*` 通配声明
src-tauri/src/ipc.rs          # 新增 write_file_base64
```

边界规则：
- **只有 `core/` 碰 `new MindMap` 与插件注册**；其他组件通过 `useMindMap()` 拿实例，通过 `bus` 收库事件，通过 `store` 共享 UI 状态。
- **`constants/` 是纯数据**，不 import Vue。
- 每个侧栏 / 弹层 / 对话框一个文件，自己订阅需要的库事件、自己在 `onBeforeUnmount` 退订。
- `MindMapEditor.vue`（`src/editor/`）**保留**：`NoteEditor.vue` 的导图模式仍以异步组件引用它（面板/弹窗内的简易画布）。导图窗口不再使用它，改用 `core/createMindMap.ts`；两者读写同一份 `mindmap_data`，`MindMapEditor` 读取时同样走 `core/persistence.parse` 以兼容全量格式（它只取 `root` 渲染，不改主题结构）。

### D12：分阶段与提交

按依赖方向五个阶段，每阶段独立可用、独立提交（每完成一个组件或一组紧耦合组件提交一次，中文提交信息）：

| 阶段 | 内容 | 交付判定 |
|---|---|---|
| P0 基础 | 依赖、资源、类型声明、`core/*`、持久化（含旧格式兼容测试）、窗口布局壳、naive-ui 主题映射、Rust `write_file_base64` | 打开旧导图不丢数据；主题/结构切换后重开仍在 |
| P1 编辑主干 | 工具栏 + 节点按钮、右键菜单、快捷键表、统计、导入、导出、超链接 / 备注 / 标签 / 图片对话框 | 参考端工具栏与右键菜单每一项都能触发 |
| P2 侧栏 | 侧栏壳与触发条、结构、主题、基础样式、节点样式、设置（含水印）、大纲、图标贴纸、公式、备注侧栏 | 参考端六个侧栏全部可用 |
| P3 画布层 | 导航工具栏、小地图、缩放、滚动条、搜索替换、全屏、演示、鼠标行为、富文本浮动栏、图标浮动栏、图片位置浮动栏、备注悬浮、图片预览、外框 / 标签 / 关联线样式面板、大纲全屏编辑、源码编辑、节点链接、附件 | 参考端画布层所有浮层可用 |
| P4 收尾 | 拖拽导入、复制到剪贴板五种格式、禅模式、剩余边角；全量类型检查、格式检查、实机逐项对照参考端 | 功能对照表全部打勾 |

## 架构

### 数据流

```
Rust (notes.mindmap_data: TEXT)
   │ api.notes.list / save
   ▼
MindMapApp.vue ── mindmapData(string) ──► core/persistence.parse ──► { root, layout, theme, view } | root
   │                                                                          │
   │ provide(mindMap, store, bus)                                             ▼
   │                                                        core/createMindMap: new MindMap({...full data, options})
   │                                                                          │ mindMap.on(30+ 事件) → bus.emit
   ▼                                                                          ▼
chrome/ sidebars/ popups/ dialogs/ ── useMindMap() ── mindMap.execCommand / setTheme / setLayout / updateConfig ...
                                   ── bus.on('node_active' | 'data_change' | ...) 更新自身状态
   ▲
   └── data_change / view_data_change ──► persistence.serialize ──► MindMapApp: dirty + 自动保存(去抖) ──► api.notes.save
```

### 库实例配置（等价参考端 `init()`，去掉浏览器/多语言/AI 相关项）

- `fit: false`、`layout / theme / themeConfig / viewData` 来自持久化数据
- `nodeTextEditZIndex: 1000`、`nodeNoteTooltipZIndex: 1000`
- `customNoteContentShow`：show/hide 走 bus → `NoteContentShow.vue`
- `openRealtimeRenderOnNodeTextEdit`、`enableAutoEnterTextEditWhenKeydown`、`demonstrateConfig.openBlankMode`、`enableFreeDrag`、`mousewheelAction`、`mousewheelZoomActionReverse`、`createNewNodeBehavior`、`isUseHandDrawnLikeStyle`、`isUseMomentum`、`alwaysShowExpandBtn`、`enableInheritAncestorLineStyle`、`imgTextMargin`、`textContentMargin`、`openPerformance`、水印配置：来自「设置」侧栏，存 `inkling.mindmap.config`（见 D5），建实例时展开进 options，改动经 `updateConfig` 即时生效
- `iconList: [...库内置, ...贴纸组]`（贴纸组异步加载后 `updateConfig({ iconList })`）
- `useLeftKeySelectionRightKeyDrag` 来自 localConfig
- `customHandleClipboardText`：移植参考端 `utils/handleClipboardText.js`
- `handleIsSplitByWrapOnPasteCreateNewNode`：naive-ui `dialog.warning` 二次确认
- `errorHandler`：`export_error` → Toast「导出失败」，其余记 `logger.error`
- `addContentToFooter`：导出底部文字
- `expandBtnNumHandler: num >= 100 ? '…' : num`
- `beforeDeleteNodeImg`：确认对话框
- `defaultNodeImage`：图片加载失败占位 SVG
- `initRootNodePosition: ['center', 'center']`
- `customInnerElsAppendTo: null`

可选插件：`RichText`（localConfig.openNodeRichText，默认开）与 `Scrollbar`（isShowScrollbar，默认关）用 `addPlugin / removePlugin` 动态挂卸，其余 18 个插件启动即注册。

### 事件总线与共享状态

- `bus.ts`：`on / off / once / emit`，类型化事件名（库事件名 + 自定义事件名联合类型），避免 vuex 与 `$bus` 两套机制。
- `store.ts`：`reactive` 对象，字段：`activeSidebar`、`isReadonly`、`isZenMode`、`isOutlineEdit`、`isSourceCodeEdit`、`activeNodes`（`node_active` 回调缓存）、`isDragOutlineTreeNode`、`extraTextOnExport`；由 `useMindMap()` 一并注入。

### 快捷键

库 `KeyboardNavigation` + `keyCommand` 内置全部节点/画布快捷键（参考端快捷键表所列）。窗口级仅保留 `⌃S 保存`（现有）；库同名快捷键通过 `mindMap.keyCommand.addShortcut('Control+s', save)` 也绑一次，保证焦点在画布内时同样生效。

### 错误处理与日志

- 所有库调用失败走 `logger.error('mindmap', ...)` + Toast，不让错误吞掉编辑器。
- 导入解析失败：Toast「文件解析失败」；xmind 多画布弹选择。
- 导出：`errorHandler('export_error')` → Toast；Rust 写文件失败把错误原文 Toast 出来。
- 关键节点日志：实例创建、插件动态挂卸、保存（手动/自动）、导入导出开始与结束、主题/结构切换。

### 测试策略

- 纯函数单元测试（Vitest 不在依赖中，**不新增测试框架**；沿用项目现状：Rust 侧 `cargo test`，前端靠 `vue-tsc` 与实机验收）。因此把可测逻辑放在 Rust 能覆盖的地方以外的部分尽量做成无副作用函数，由类型系统兜底。
- Rust：`write_file_base64` 的解码与路径写入有单元测试（临时目录）。
- 实机验收：按「范围 · 做」表逐项对照参考端操作一遍，结果记录在 plan 的验收清单里。

## 未决

无。以下一点已在决策中给出默认值，实施时若库行为与预期不符，按「不展示该功能项 + 记录原因」处理，不阻塞其他部分：
- `image.js` 的 2.1MB SVG 贴纸（D9 不移植）。
