# 原型对齐 · 阶段五「思维导图窗口」设计

**日期**：2026-09-14
**前置**：阶段一（样式分层 / 动效）、阶段二（D10–D14）、阶段三（D15–D21）、阶段四 A/B/C（D22–D40）；以及**思维导图全功能线**（`../plans/2026-09-06-mindmap-full-features.md`，35 个任务已完成，`69a3aea` 有验收记录）
**原型**：`docs/index.html` `#mindmapWindow`（304–435）、`#mindmapCtxMenu`（599–611）、`#mmExportOverlay`（614–651）；`docs/app.js` 导图段（1616–2600 前后，函数表见 `.superpowers/sdd/recon-phase5-prototype.md`）；`docs/styles.css` 导图段（460–770 令牌化段、`:root` 的 `--mm-*` 101–113、typewriter 覆盖 2564 起）
**勘察存档**：`.superpowers/sdd/recon-phase5-prototype.md`、`.superpowers/sdd/recon-phase5-current.md`（本文档的行号引用均出自这两份）

## 1. 范围

把**已经完备**的导图窗口（参考项目移植线，70 个文件）的**外壳**对齐到原型形态，并兑现「导图窗口接入全部主题」。

**本阶段的特殊性**：原型导图窗口是**同一个库（`simple-mind-map`）的简化演示外壳**，其中 **16 处控件是演示桩**（格式刷/关联线只 toast、小地图是固定假图、搜索不定位、外框三个下拉取值被丢弃、AI 插两个写死节点、4 个「保存格式」后缀只是字符串……）。而我们的实现是真功能（库原生能力 + 官方插件）。
⇒ **本阶段的一条硬规则（D47）：形态照原型，能力用我们的；凡是原型为桩而我们有真实现的，一律保留我们的实现，不降级。**

**不在本阶段**：AI（全功能线的「不做」表已含；本阶段左岛不放 AI 按钮）；导出相关新 UI（D41）；协同编辑；浏览器本地文件目录树（同前）。

## 2. 决策记录（本阶段新增，均已确认）

| # | 问题 | 采用 | 理由 |
|---|---|---|---|
| D41 | 导出边界 | **保留现有导图导出**（工具栏「导出」→ `ExportDialog`，可导 SMM/PNG/SVG/PDF/Markdown），阶段五**不新增**导出 UI（原型那个 `#mmExportOverlay` 不做） | 09-09 那轮已完成并验收的能力不回收；用户所说「不需要导出」指不为对齐原型再做一个 |
| D42 | 图标 / 公式的组织 | **改成工具栏按钮 → 模态**（与原型一致；备注已是模态） | 对齐原型形态；抽屉只留原型那 6 项 |
| D43 | 快捷键侧栏 | **保留**（我方扩展，原型无对应物；入口仍在底栏「更多」下拉） | 看快捷键方便，且不占 dock |
| D44 | 文件名岛的范围 | **只做重命名**（显示 + 点 ✏️/双击进入编辑），**不做「保存格式」下拉** | 我们的导图存进笔记（`mindmap_data`），没有「保存格式」概念；原型那 4 个后缀实测只是字符串、落库无差异 |
| D45 | 样式承载 | 组件**改用原型的类名与结构**，让**生成层** `components.css:652-1484`（原型导图 CSS，含 104 处 `--mm-*`）直接接管；自有层 `mindmap.css` 收敛为「真实窗口适配 + 项目扩展」 | 与阶段二/三/四同一条路；生成层已完整具备原型样式，自造一套等于重复维护 |
| D46 | 主题接入 | 给其余 30 套主题补 `--mm-*` 覆盖（照 `themes.css:1143` 的 typewriter 块模式，该块在 `:1188-1200`）；同时修 `naiveTheme.ts` 的两处硬编码 | **原型源码自己写明了这是它的待办**：`docs/styles.css:95-100` 注释自陈该窗口原先散布 139 处硬编码色值、是「全站唯一不跟随主题换肤的窗口」，本轮只完成令牌化（默认值刻意等于原硬编码值以求零回归），并标注「⚠️ 下一步：在各 `[data-theme]` 主题块中覆盖以下 13 个令牌」（记在需求文档 6.4 待办）。阶段五即兑现这一步 |
| D47 | 桩的对策 | **形态照原型、能力用我们的**（对照表见 §3「原型桩」列） | 照抄会掉能力（详见 §1 本阶段特殊性） |
| D48 | 未保存修改 | **保留我们的**：`dirty` 时关窗 `confirm('思维导图尚未保存，确认关闭？')`（`MindMapApp.vue:207`） | 原型直接丢弃修改（✕ 与「取消」都没有提示），照抄是降级 |
| D49 | 内部命名 | 状态 key 沿用我们的 `structure` / `setting`（UI 文案与原型一致：「结构」「设置」） | 原型用 `data-side="struct"/"settings"` 只是它自己的 DOM 属性；我们的 key 更可读，且已被 10 个侧栏与 `store.ts` 引用，改名收益为零 |
| D50 | 图标字形 | **沿用现有 iconfont 矢量图标**，不改成原型的 emoji（`<span class="mm-ico">💾</span>`） | 形态（三岛结构）照原型，字形技术沿用现状：换 emoji 会让整个窗口的图标语言与现状不一致，属纯外观变动、不在本阶段范围。新增的「保存 / 新建 / 关闭」三枚取自 `src/assets/mindmap/icon-font/iconfont.css` 实有清单（`iconlingcunwei` / `iconxinjian` / `iconguanbi`），**不得凭印象造类名** |
| D51 | 导图窗口默认尺寸与顶栏降级 | 默认 **1360×900**；窄窗口下**左岛内部横向滚动、中岛与右岛不压缩**（右岛的保存/导出/关闭永不被裁） | 三段式顶栏按生成层实际字号（10.5px）实测需 **1325–1525px** 才排得下一行（左岛 17 枚 893px + 右岛 5 枚 256px + 中岛 120–320px + `.mm-topbar` gap/内距 56px，GDI+ 实测），原型的 `min(97vw, 1180px)` 在自家宽度下同样溢出（`.mm-island` 是 `overflow-x:auto`）。1360 留余量且放得进 1706×1066 的工作区；降级规则保证拉窄时关键按钮仍可见 |
| D52 | 「新建」的语义 | **换窗口**：confirm 后先开 `mindmap-new`、再关本窗（不就地清空） | 窗口 label（`mindmap-<笔记id>`）是「一笔记一窗口」的唯一保证且 Tauri v2 不可变；就地清空会让本窗顶着旧笔记的 label 显示新导图——那条笔记的导图在关窗之前打不开，且新建保存后从主窗口再打开新笔记会**另开一窗、两窗同编一条笔记互相覆盖**（Task 2 审查发现，Important） |

## 3. 与原型的差异处理表

| # | 项 | 原型 | 现状 | 做法 |
|---|---|---|---|---|
| **顶栏** | | | | |
| 1 | 结构 | `.mm-topbar` > 左岛 `.mm-island-left`（16 按钮）+ 中间 `.mm-filename-island` + 右岛 `.mm-island-right`（5 按钮） | `Toolbar.vue`（顶部一整条，14 个节点项 + 导入/导出/回退/前进/格式刷）；保存/关闭在自有标题栏 | 按原型重排为三岛结构（`index.html:311-359`）；用生成层 `.mm-topbar`/`.mm-island`/`.mm-tool-btn` 等规则 |
| 2 | 左岛按钮 | 16 个：回退/前进/格式刷/同级/子级/删除/图片/图标/超链接/备注/标签/概要/关联线/公式/外框/**AI**（删除项带 `.danger`，悬停变红） | `Toolbar.vue` 节点组 14 项（多「链接节点」「附件」，无 AI；删除项**未套 `danger` 类**，故生成层 `.mm-tool-btn.danger:hover`（`components.css:788`）一直空转） | 采用原型的 15 项（**不放 AI**，D41/§1）；**保留**我们的「链接节点」「附件」两项（真能力，原型没有）；**给删除项补 `danger` 类**，让那条既有规则生效 |
| 3 | 文件名岛 | `#mmFilenameText` + `#mmFilenameInput` + `#mmExtBadge(.smm)` + `#mmExtSelect` + `#mmFilenameEditBtn` ✏️/💾 | **无**（标题栏一行静态 `🧠 {{ title }}`） | 新做，但**只做重命名**（D44）：不渲染 `#mmExtBadge`/`#mmExtSelect` |
| 4 | 右岛 | 保存 / 新建 / 导入 / 导出 / 取消 | 保存、关闭在标题栏；导入、导出在 `Toolbar`；**「新建」没有**；没有「取消」 | 归位到右岛；**新增「新建」**（confirm 后重置为空白导图，原型的 `mmNew` 语义）；「取消」不单独做——我们 1500 ms 自动保存（`MindMapApp.vue:170-173`），除自动保存窗口内没有可"放弃"的修改，「关闭」+ `dirty` 确认已覆盖（D48）；**导出保留**（D41） |
| 5 | 标题栏 | `.window-titlebar`（🧠 标题 + ✕） | `header.mindmap-bar`（标题 + `TagList` + 关闭 + 保存） | 保留自有标题栏的 `TagList`（原型没有标签预览，属项目扩展），按钮按 #4 迁走 |
| **画布与右侧** | | | | |
| 6 | dock | `.mm-sidebar-dock` 6 项（nodeStyle/baseStyle/theme/struct/outline/settings） | `SidebarTrigger.vue` **6 项且顺序一致** | **已一致**；改用生成层 `.mm-dock-item` 样式（D45） |
| 7 | 抽屉 | **单个** `#mmDrawer`（换 title/body 内容） | `SidebarShell.vue` × **10 个实例**同存 DOM（`activeSidebar` 单值做视觉互斥） | 收敛为单壳（一个 `.mm-drawer`，body 按 `activeSidebar` 切内容） |
| 8 | 图标 / 公式 | 左岛按钮 → 通用模态 `#mmModalOverlay` | 右侧抽屉（`IconSidebar` / `FormulaSidebar`） | **改成模态**（D42），用生成层 `.mm-modal*` + `.mm-icon-grid` / `.mm-formula-table` |
| 9 | 备注 | 左岛按钮 → 通用模态 | `NodeNoteDialog.vue`（已是模态） | 已一致；改用生成层 `.mm-modal*` 样式 |
| 10 | 快捷键表 | **无对应物** | `ShortcutSidebar`（底栏「更多」入口） | 保留（D43） |
| 11 | 死码 | — | `NoteSidebar.vue` 无任何入口（`activeSidebar='nodeNoteSidebar'` 全仓无写入） | **删除**该组件与 `SidebarName` 里的取值 |
| **底栏** | | | | |
| 12 | 结构 | `.mm-bottombar`：左 `#mmStatLeft`（字数/节点数）+ 右 12 控件（语言下拉（死控件）/定位中心/搜索/只读/小地图/全屏 ｜ 缩小/百分比/放大/自适应/展开全部/收起全部） | `Count.vue`（左下）+ `NavigatorToolbar.vue`（右下，含回到根节点/搜索/鼠标行为/小地图/只读/全屏/缩放/演示/更多） | 按原型排布到 `.mm-bottombar`；**不放语言下拉**（死控件）；保留我们的「鼠标行为」「演示」（真能力） |
| 13 | 定位中心 vs 自适应 | 两个入口同一个 `view.fit()` | 我们也是两个（回到根节点 / 自适应） | 保留（无害，且原型的「展开全部/收起全部」我们也有） |
| **浮层** | | | | |
| 14 | 右键菜单 | `#mindmapCtxMenu`（**body 级** fixed，10 项 + 2 分隔线，避免被窗口裁剪）；样式用**通用令牌**（`--wsa`/`--text-strong`/`--text-dim`/`--red-soft`） | `ContextMenu.vue`（挂在 `.mm-stage` 内） | **改挂 body 级**（原型的理由成立：窗口 `overflow`/`transform` 会裁剪）；**令牌改用 `--mm-*`**——原型此处用通用令牌，会导致「右键菜单跟随 31 套主题、而窗口外壳跟随 `--mm-*`」两者不同步（原型自身的不一致）；本阶段窗口内部统一走 `--mm-*` |
| 15 | 搜索 | `.mm-search-box` 药丸框；回车**只 toast 命中/未命中**（桩） | `SearchBox.vue` + **真定位/高亮** | 形态照原型，**交互保留我们的**（D47） |
| 16 | 小地图 | `.mm-minimap`；**固定假图**，不随数据变（桩） | `Navigator.vue` 真缩略图（订阅 `toggleMiniMap`） | 形态照原型，**实现保留我们的**（D47） |
| 17 | 通用模态遮罩 | `#mmModalOverlay` 一处复用（8 种表单） | 各自独立对话框（`NodeImageDialog`/`NodeHyperlinkDialog`/`NodeNoteDialog`/`NodeTagDialog`/`NodeLinkDialog`/…） | 图标/公式改模态时**复用同一个遮罩壳**（新抽一个 `MmModal.vue`），其余对话框保持独立（内容差异大） |
| **主题与样式** | | | | |
| 18 | `--mm-*` 令牌 | `:root` 13 个（`styles.css:101-113`，白底/Ant 蓝）+ **仅 typewriter** 覆盖（`:2564`） | 生成层逐字一致：`tokens.css` 13 处（`:root`）+ `themes.css:1188-1200`（typewriter 块内） | 给**其余 30 套主题**补 `--mm-*` 覆盖（D46） |
| 19 | chrome 样式来源 | 生成层（`components.css:652-1484`，104 处 `--mm-*`） | 自有层 `mindmap.css`（1685 行，156 处用 `--text/--wsa/--accent` 等通用令牌，仅 14 处硬编码） | 改用原型类名 → 生成层接管；`mindmap.css` 收敛为窗口适配 + 项目扩展（D45） |
| 20 | naive-ui 主题 | — | `naiveTheme.ts`：基底恒 `darkTheme`；`inputColor`/`borderColor` 硬编码白色半透明 | 浅色主题下改用 `lightTheme` 基底；`inputColor`/`borderColor` 改由令牌推导（D46） |
| **保留项（原型为桩 / 我们更完整）** | | | | |
| 21 | 格式刷 | 仅 toast（`app.js:2072`） | `Toolbar` 有「格式刷」入口（真实现） | 保留 |
| 22 | 关联线 | 仅 toast | 真实现（`AssociativeLineStylePanel` 等） | 保留 |
| 23 | 外框 | 三个 select 取值被丢弃（`app.js:2067`） | `OuterFramePanel` 真实现 | 保留 |
| 24 | 设置抽屉 | 7 个 checkbox 里 6 个无监听 | `SettingSidebar` 282 行真实现 | 保留 |
| 25 | 保存内容 | `getData(false)` 只存节点树 ⇒ 结构/主题/画布背景/视口全丢 | `getData(true)` 全量（D4） | 保留 |
| 26 | 未保存修改 | 直接丢弃 | `dirty` + 关窗 confirm（D48） | 保留 |
| 27 | 导入 | 只 `accept=".json"` | ImportDialog 支持 `.smm/.json/.xmind/.md` + 拖拽 | 保留 |

## 4. 设计

### 4.1 顶栏三岛（`chrome/Toolbar.vue` → 拆为 `MmTopbar.vue` + `MmFilenameIsland.vue`）

- 模板骨架（类名用生成层已有的）：

```vue
<div class="mm-topbar">
  <div class="mm-island mm-island-left">
    <button v-for="b in leftButtons" class="mm-tool-btn" :class="{ danger: b.danger }" :title="b.title" :disabled="b.disabled?.()" @click="b.run">
      <i class="iconfont" :class="b.icon" /><span class="mm-txt">{{ b.label }}</span>
    </button>
  </div>
  <MmFilenameIsland :name="title" @rename="renameTo" />
  <div class="mm-island mm-island-right"><!-- 保存 / 新建 / 导入 / 导出 / 取消 --></div>
</div>
```

- `leftButtons`：原型 15 项（去掉 AI）＋ 我们的「链接节点」「附件」；顺序按原型（回退/前进/格式刷/同级/子级/删除/图片/图标/超链接/备注/标签/概要/关联线/公式/外框），新增两项插在「超链接」之后。
- 禁用态沿用现有 `Btn` 结构（`disabled?: () => boolean`）；原型无禁用概念（点了没反应）。

### 4.2 文件名岛（`MmFilenameIsland.vue`，新）

**注意**：现状 `MindMapApp.vue:102` 的 `title` 是**静态文案**（`'新建思维导图'` / `'编辑思维导图'`），根本不显示笔记名——所以这一块不只是「可编辑」，而是**首次把导图名显示出来**。

- 常态：`<span class="mm-filename-text">{{ name }}</span>` + `<button class="mm-filename-btn">✏️</button>`；点击文本或按钮进入编辑态。
- 编辑态：`<input class="mm-filename-input" maxlength="32">` + `<button class="mm-filename-btn">💾</button>`；Enter / 失焦 / 点 💾 提交，Esc 取消。
- 提交：写回笔记标题并同步主窗口列表；空名回退为「未命名导图」（原型的处理，`app.js:1697-1698`）。
- 不渲染 `#mmExtBadge` / `#mmExtSelect`（D44）。
- **标题存哪个字段**：实现时按 `core/persistence.ts` 与 `notes.save` 的现状确认（导图数据在 `mindmap_data`，笔记标题在 `content`），并在组件注释里写明——本 spec 不预设。

### 4.3 单抽屉壳（`sidebars/SidebarShell.vue` 改造）

- `MindMapApp.vue` 里 10 个 `<XxxSidebar />` 改为**一个** `<MmDrawer>`，内部按 `ui.activeSidebar` 用 `<component :is>` 或 `v-if` 切换；只有 6 个 dock 项对应的侧栏可被打开（其余 4 个改走模态/下拉，见 D42/D43）。
- 样式改用生成层 `.mm-drawer` / `.mm-drawer-header` / `.mm-drawer-body`（`components.css:892-943`）。

### 4.4 图标 / 公式模态（`popups/MmModal.vue` 新抽）

- 结构与原型 `#mmModalOverlay`（`index.html:398-410`）一致：遮罩 + `.mm-modal`（header: 标题+✕ / body / footer: 取消+确定）。
- 图标：`.mm-icon-grid` + `.mm-icon-tile`（`components.css:1103-1127`），4 组（优先级 / 进度 / 表情 / 标记），**保留我们的库原生图标集**（原型是内置 emoji 列表）。
- 公式：`.mm-formula-table`（`components.css:1128-1144`），保留我们的 LaTeX 输入与常用公式表。
- 两者原本的 `bus.emit('showNodeIcon' / 'showNodeFormula')` 事件名不变，只换取渲染宿主。

### 4.5 底栏（`chrome/NavigatorToolbar.vue` + `Count.vue` → `MmBottombar.vue`）

- 左：`#mmStatLeft` 对应 `Count.vue` 现有内容（字数 / 节点数）→ 用生成层底栏样式。
- 右：按原型顺序（定位中心 / 搜索 / 只读 / 小地图 / 全屏 ｜ 缩小 / 百分比 / 放大 / 自适应 / 展开全部 / 收起全部），把 `NavigatorToolbar` 现有按钮重排；**不放语言下拉**（原型死控件）；保留「鼠标行为」「演示」两项（真能力，插在全屏之后）。
- 高亮态（只读/小地图开时）用生成层 `.mm-ctrl-btn.active`（若生成层没有则补一条自有层规则，注明理由）。

### 4.6 右键菜单挂 body（`popups/ContextMenu.vue`）

- 用 `Teleport to="body"`（`CardConfirm.vue` 已是这个模式），样式用生成层 `.mm-ctxmenu`（`components.css` 内）；定位沿用现有逻辑，只改挂载点。

### 4.7 主题接入（D46）

- 在 `themes.css` 的**每个**主题块里补 13 个 `--mm-*`（照 `:1188-1200`），取值从该主题已有的 `--menu-bg` / `--text` / `--text-dim` / `--accent` / `--body-bg` 推导（浅色主题用浅色 surface、深色主题用深色 surface）。
- 修 `naiveTheme.ts`：
  - 基底按主题明暗选 `darkTheme` / `lightTheme`（需要知道主题明暗：`constants/themes.ts` 里应能判定，或按 `--body-bg` 亮度算）；
  - `inputColor` / `borderColor` 改由 `--wsa` / `--text-dim` 推导。
- 画布（simple-mind-map 的导图主题）**不随窗口主题自动切换**（它是文档数据的一部分，D4 存进 `mindmap_data`）——本阶段只保证 chrome 跟随。

### 4.8 清理

- 删 `sidebars/NoteSidebar.vue` 与 `SidebarName` 的 `nodeNoteSidebar`（#11）。
- `mindmap.css` 里被生成层取代的规则删除；保留：真实窗口适配（无边框窗口的圆角/拖拽区）、项目扩展（标签预览、拖拽导入遮罩、禅模式）、生成层未覆盖的新组件（`MmFilenameIsland` / `MmModal` / `MmBottombar` 的少量补丁）。
- 顺带清掉 4C 验收记录 §5.1 的两处「失败仍报成功」（`persist` 与 `saveLauncherRoots`）。

## 5. 测试

- 纯函数：文件名岛的名称裁剪/提交逻辑若抽函数则测（`maxlength=32`、空名回退）。
- 主题：加一条单测断言「每套主题都定义了全部 13 个 `--mm-*`」（读 `themes.css` 文本或用 `getComputedStyle` 的 node 侧替身）——防止以后新增主题漏配。
- 其余为组件与视觉，走实机。
- 全套：`pnpm typecheck` / `pnpm test` / `pnpm sync:styles --check` / `format:check` / `cargo fmt --check` / `cargo test` / `vite build`。

## 6. 实机验收（打字机 + 深色 + 一套浅色主题）

1. 顶栏三岛：左岛按钮齐全（15 原型项 + 链接节点 + 附件）、文件名岛显示与改名生效（改完主窗口笔记列表同步）、右岛五钮归位。
2. 抽屉：dock 六项各开一次，单壳切换无残留；图标/公式走模态；快捷键侧栏仍能从底栏「更多」打开。
3. 底栏：12 个控件的功能逐个生效（定位/搜索/只读/小地图/全屏/缩放/自适应/展开收起），鼠标行为与演示保留。
4. 右键菜单：在窗口边缘（画布右下角）右键，菜单**不被窗口裁剪**；项数与功能与现状一致。
5. 主题：切「打字机 / 一套深色 / 一套浅色」三套，导图 chrome（顶栏/抽屉/模态/底栏/右键菜单）跟着变且可读；**浅色主题下 naive 控件不再是暗底**（本次修的瑕疵）。
6. 能力回归：格式刷 / 关联线 / 外框 / 设置抽屉 / 小地图 / 搜索定位 / 导入（.smm/.json/.xmind/.md）/ 导出（现有）**均不受外壳重排影响**。
7. 未保存提示：改一笔后关窗 → 弹确认；保存后关窗 → 不弹。
8. 深色与浅色下 `NoteSidebar` 删除后无残留引用；`sync:styles --check` 通过（未手改生成层）。

## 7. 提交批次（中文）

1. `feat(mindmap): 顶栏按原型改为双工具岛 + 新建文件名岛（重命名）`
2. `feat(mindmap): 侧栏壳收敛为单抽屉、图标与公式改为模态`
3. `feat(mindmap): 底栏按原型重排、右键菜单改挂 body`
4. `feat(mindmap): 其余 30 套主题接入 --mm-* 令牌，修 naive 浅色基底与输入框配色`
5. `refactor(mindmap): 自有层样式收敛为窗口适配与项目扩展（改用生成层原型类名）`
6. `chore(mindmap): 删除 NoteSidebar 死码、清理 4C 遗留的失败报成功`
7. `docs(design): 阶段五实机验收记录`

## 8. 风险

| 风险 | 应对 |
|---|---|
| 改用原型类名后，生成层规则与现有组件结构不匹配（生成层是按原型的 DOM 层级写的） | 分批替换：先顶栏（最独立），每批跑实机看观感；自有层里保留旧规则直到该批验收通过 |
| naive-ui 换 `lightTheme` 基底影响所有 naive 控件（颜色选择器 / 滑块 / 树…） | 只改基底与两个硬编码令牌，逐个主题实机核对；必要时按主题明暗自适应 |
| 单抽屉壳改造波及 10 个侧栏 | 先做壳，侧栏内容组件不改（只换宿主）；`activeSidebar` 语义不变 |
| 右键菜单改挂 body 后定位坐标系变化（原本相对 `.mm-stage`） | 定位一律用 `clientX/clientY`（body 级 fixed）；在窗口四角各测一次 |
| 主题补 30 套 × 13 个令牌 = 390 个值的观感 | 取值从各主题既有令牌推导而非手挑；验收按「深色 / 浅色 / 打字机」三类各查一套 |
| 300 行的 `SettingSidebar` 等大组件在换壳后布局错位 | 换壳不改内容；验收第 6 项专门回归 |

## 9. 实机结论（实施后填写）

- 顶栏三岛与文件名岛：待填。
- 单抽屉壳与模态：待填。
- 主题接入（30 套）：待填。
- 补丁清单：待填。
