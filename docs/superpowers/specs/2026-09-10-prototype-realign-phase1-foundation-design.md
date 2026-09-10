# 原型对齐 · 阶段一「基础层」设计

**日期**：2026-09-10
**状态**：已与用户逐节确认，待用户复核全文
**唯一参考原型**：`docs/index.html` + `docs/styles.css` + `docs/app.js`（`doc/` 为旧原型，仅作历史参考，不再对齐）

---

## 1. 背景与目标

当前 `src/` 是按旧原型 `doc/` 落地的（见 `docs/superpowers/plans/2026-09-01-inkling-prototype-parity.md`）。新原型 `docs/` 此后大幅演进：

| 维度 | 新原型新增 / 变化 | 当前应用状态 |
|---|---|---|
| 令牌 | 尺度令牌体系（间距 / 字阶 / 行高 / 圆角阶 / 阴影阶 / 动效 / 层级 / 焦点环）、13 个 `--mm-*` 导图语义色 | 只有颜色令牌 + 少量自建动效令牌 |
| 质量层 | 焦点环、仅悬浮可见操作的 focus-within 等价、禁用态、reduced-motion、细滚动条、选区色 | 分散、部分缺失 |
| 主题 | 默认主题改为「打字机」（米色纸面 + 黑描边白卡 + 打字机红 + 等宽字体 + 锐利直角 + 无毛玻璃），共 30 套 | 默认深色，无打字机，共 31 套（含自建棕褐） |
| 交互 | Zen 专注模式、侧边栏五页签、启动台页 / 灵动岛页、思维导图窗口新布局等 | 未实现 |
| 动画库 | 面板滑入滑出 gsap | gsap；用户要求改用 animejs |

选择器级差异：新原型 732 条，旧原型 463 条，新增 276 条；当前应用 628 条，其中 190 条为项目自建（不在任一原型中）。

**总目标**：按新原型重新对齐全部窗口的主题样式与交互逻辑；以 ui-ux-pro-max 的 UX 准则做可达性与动效节奏审查；用 animejs 实现原型已有动效与微交互。**原型为准，只加微交互，不改变原型的视觉语言。**

**阶段一目标**：打好地基——样式四层从新原型重新生成、打字机默认主题、animejs 动效模块替换 gsap、列表错落入场、同步脚本。不改任何窗口的页面结构。

---

## 2. 决策记录

每条均已向用户提问并得到确认。

| # | 问题 | 采用 | 理由 |
|---|---|---|---|
| D1 | 参考哪套原型 | `docs/`（新） | 用户在 IDE 打开的即此套；含全部最新设计 |
| D2 | 对齐范围 | 全部 7 类窗口，分阶段推进 | 每阶段独立 spec → plan → 实现 |
| D3 | 「严格参考原型」与「美化」的边界 | 颜色 / 字体 / 尺寸 / 布局 / 交互逻辑全部以原型为准；ui-ux-pro-max 只用于可达性、交互状态、动效节奏审查；animejs 只实现原型已有动效与不改变视觉语言的微交互 | 避免两个来源打架 |
| D4 | gsap 处理 | 全部迁移到 animejs v4，卸载 gsap | 单一动画库，包体更小、节奏统一 |
| D5 | 默认主题 | 打字机为默认，与原型一致 | 严格对齐原型 |
| D6 | 项目自建扩展（棕褐主题、玻璃三档 `data-glass`、主窗口毛玻璃开关 `data-acrylic`） | 全部保留 | 与原型不冲突，只是叠加在原型令牌之上 |
| D7 | 样式移植做法 | 整体重新移植 + 同步脚本；自建样式收拢到独立 `extensions.css` | 现有切分文件已被 prettier 重排，增量打补丁对不上；脚本让日后原型更新一条命令重同步 |
| D8 | 旧库默认主题 | 做 v5 迁移：`theme = 'dark'` 一次性改为 `typewriter` | 建库时就写入了 `dark`，不迁移则老安装永远看不到新默认；与 v4 面板位置迁移同一取舍（无法区分主动选择与建库默认，主动选过深色的用户需重选一次） |
| D9 | 列表错落入场 | 阶段一即接入主窗口与面板的六处列表 | 用户要求 |

---

## 3. 总体拆分（五个阶段）

| 阶段 | 内容 | 状态 |
|---|---|---|
| 一 · 基础层 | 本文档 | 设计完成 |
| 二 · 主窗口 | 侧边栏五页签（笔记 / 粘贴板 / 待办 / 启动台 / 灵动岛）+ 底部设置 / 统计、启动台页、灵动岛页、偏好设置页、统计、日期详情、主题下拉；视图切换与列表动效精修 | 待设计 |
| 三 · 呼出面板 | 矢量三态圆点导航、Zen 专注模式、编辑器底栏、粘贴板 / 待办页；滑入滑出精修 | 待设计 |
| 四 · 小窗口 | 浮窗启动台、灵动岛（流光边框 / 尺寸手柄 / 提醒岛内呈现）、置顶浮窗、提醒卡片、标签管理 / 待办编辑 / 粘贴板编辑弹窗、二次确认浮层、toast、感应区 | 待设计 |
| 五 · 思维导图窗口 | 顶部双工具岛、文件名岛、右侧 dock 与抽屉、底部控制栏、导出对话框、右键菜单；导图窗口接入全部主题 | 待设计 |

阶段二到五各自在设计时再读取原型对应段落，本文档不预设其细节。

---

## 4. 样式分层架构

加载顺序（`src/styles/index.ts`，不可调换）：

| 序 | 层 | 文件 | 来源 | 内容 |
|---|---|---|---|---|
| 1 | 令牌 | `tokens.css` | **脚本生成** | 原型文件头至「全局质量层」之前：`* {}`、`:root` 全部令牌（尺度 + 深色基础主题 + `--mm-*`）、`body`、`.hidden`、`.glass`、`kbd`、`button`、`.btn*` |
| 2 | 质量层 | `base.css` | **脚本生成** | 原型「全局质量层」：`:focus-visible` 焦点环、`:focus-within` 显隐、`:disabled`、`prefers-reduced-motion`、滚动条、`::selection` |
| 3 | 组件 | `components.css` | **脚本生成** | 原型「hotzone」至「多主题系统」之前的全部组件样式，自动剔除演示脚手架 |
| 4 | 主题 | `themes.css` | **脚本生成** | 原型「多主题系统」至文件末：30 套 `[data-theme]`（含打字机及其质感补丁）、随段落一起进入的 `.theme-dd` 主题下拉样式 |
| 5 | 项目扩展 | `extensions.css` | 手写（新建） | 见第 6 节 |
| 6 | 窗口适配 | `window-fit.css` | 手写（已有） | 原型浮层定位 → 真实窗口铺满；本阶段只在验收发现破损时做最小补充 |
| 7 | 动效 | `motion.css` | 手写（已有） | 只含 transition 的状态过渡层；本阶段不改 |
| 8 | 玻璃档位 | `glass.css` | 手写（已有） | `data-glass` 三档；本阶段不改 |

窗口专属样式（`island.css`、`launcher.css`、`mindmap.css`、`mindmap-outline.css`）仍由各自入口单独引入，位于第 8 层之后；本阶段不改。

**铁律**
- 脚本生成的四个文件**永远不手改**。文件头固定写：生成来源、生成命令、原型 SHA-256、「勿手改」。
- 项目自有样式只能进 `extensions.css` 及其后各层。
- CSS 只用令牌表达颜色；脚本生成物按原型原样保留（含原型内的裸色值）。

---

## 5. 同步脚本 `scripts/sync-prototype-styles.mjs`

**输入**：`docs/styles.css`。**输出**：`src/styles/{tokens,base,components,themes}.css`。

**切分规则**：按原型的分节标题注释（形如 `/* ═══ 标题 ═══ */`）定位，不依赖行号。

| 段 | 起点（标题含） | 终点（下一标题含） | 去向 |
|---|---|---|---|
| 令牌 | 文件头 | 「全局质量层」 | tokens.css |
| 质量层 | 「全局质量层」 | 「桌面环境」 | base.css |
| 演示 | 「桌面环境」 | 「hotzone」 | **丢弃** |
| 组件 | 「hotzone」 | 「多主题系统」 | components.css |
| 主题 | 「多主题系统」 | 文件末 | themes.css |

若任一标题找不到，脚本报错退出，不产出半成品。

**演示脚手架过滤**（对全部四段生效）：按规则逐条解析 `selector-list { ... }`，对逗号分隔的选择器逐个匹配拒绝名单——精确项 `#desktop #menubar #clock #trayIcon #trayMenu #fakeApp #onboarding #demoBar .dot`（允许后接 `.`/`:`/`[`）与前缀项 `.tray-icon .fake- .onboard- .demo- .menubar-`。匹配时把选择器按组合符切成复合选择器，**任一复合选择器命中即删**：这样 `[data-theme="typewriter"] #demoBar` 也能被识别；而 `.nav-dot.dot-note`、`.di-dot.high` 的复合选择器以 `.nav-dot`/`.di-dot` 起始，不受 `.dot` 影响。逗号列表中只删除匹配项，其余保留；整条规则的选择器全部被删时整条丢弃。`@media` 块内部递归处理。已知需要保留的两处：灰度列表（原型 :802）删 `.onboard-title` 后其余保留；打字机主题的 `backdrop-filter: none` 列表删 `#demoBar` 后其余保留，`[data-theme="typewriter"] #menubar` 整条丢弃。

**规范化**：输出经 prettier（项目 `.prettierrc.json`）格式化，与仓库其他 CSS 一致。

**文件头**：

```css
/* 由 scripts/sync-prototype-styles.mjs 从 docs/styles.css 生成，勿手改。
   重新生成：pnpm sync:styles   校验：pnpm sync:styles --check
   原型 SHA-256：<hash>   分段：<段名> */
```

**内置检查**：
1. 令牌唯一性：tokens.css 的 `:root` 内每个 `--` 变量只定义一次；
2. `--check` 模式：把生成结果与仓库中现有文件逐字比较，不一致则非零退出并列出差异文件；
3. 生成后统计并打印每段规则数与被过滤的选择器清单，便于人工复核。

**package.json 脚本**：`"sync:styles": "node scripts/sync-prototype-styles.mjs"`。

**首次生成的一次性核对**：用第 1 节的选择器集合脚本再跑一次，确认「新原型选择器在应用中缺失」清零（除被过滤的演示项），「应用自建选择器」全部落在 extensions.css 及其后各层。

---

## 6. 项目扩展层 `extensions.css`

新建文件，把当前散落在生成层里的自建样式集中过来，按来源分节并注明：

| 分节 | 迁自 | 内容 |
|---|---|---|
| 1. 扩展令牌 | 现 `tokens.css` | `:root { --dur-instant: 90ms; --ease-in-out; --lift: 2px; --stagger: 30ms; --glass-blur: 24px; --glass-saturate: 160%; --glass-border-width: 1px }`；`.glass` 改为引用玻璃令牌（否则 `glass.css` 的 data-glass 三档失效）；reduced-motion 下扩展时长归零 |
| 2. 交互修正与窗口基础 | 现 `base.css` | 输入区 / `.ProseMirror` / `.cm-editor` 恢复 `user-select: text`；`[data-tauri-drag-region]` 拖拽区与 `no-drag`；`:root[data-acrylic='off']` 令牌降级与 `.glass` 去模糊；`#app` 铺满窗口 |
| 3. 同名规则的项目补充声明 | 现 `components.css` | 只写项目追加的声明，不重复原型的值：`.window-titlebar { overflow: visible }`、`.te-field input/select { width: 100%; min-width: 0 }`、`.prio-opt { min-height: 32px; position: relative; outline: none }` |
| 4. 错落动画态 | 新增 | `.is-animating { transition: none !important; }`（JS 动效期间禁用 CSS transition，避免拖影） |
| 5. 过渡桥接（阶段四迁移后删除） | 旧原型 `doc/styles.css` | ConfirmPopover / TodoEditorModal 仍按旧原型结构实现，新原型已改为全局 `#cardConfirm` 与锚定的 `#todoEditorPanel`：桥接 `@keyframes confirmIn`、`.card-confirm-text`、`#todoEditorOverlay` 的居中遮罩声明，以及随第 6 节抽取进来的 `.card-confirm`、`.todo-item:has(.card-confirm) .todo-del`、`#todoEditorModal`、`#todoEditorModal .search-input` |
| 6. 自建组件样式 | 现 `components.css`（脚本抽取：所有选择器都不在新原型里的规则，共 93 条） | 感应区指示器 `:root[data-window='hotzone']`、`.hotzone-indicator*`、`#hotzone[data-edge=…]`；笔记工具栏与类型徽章 `.notes-toolbar*`、`.note-kind`、`.kind-*`、`.archive-item.mindmap`；窗口控制按钮 `.win-controls*`、`.mac-dot*`、`.win-btn*`、`:root[data-maximized]`；设置页扩展控件 `.settings-body input[type=…]`、`.setting-section-title`、`.setting-col*`、`.launcher-roots*`、`.setting-hint*`、`input[type='number']`、`input[type='checkbox']`；待办编辑器扩展 `.te-field-date/time/priority`、`.te-remind-channels`、`.te-channel*`；优先级菜单勾选态 `.prio-opt .prio-check`、`[aria-selected='true']`、`.prio-opt.active`；标签抖动补丁 `.tag-chip.shaking*`；编辑器与代码块 `.editor .is-empty::before`、`.editor blockquote/ul/ol/hr/s`、`.pm-codeblock*`、`.note-mindmap-summary`、`.mindmap-editor*` |
| 7. 棕褐主题 | 现 `themes.css` :1085–末 | `:root[data-theme='sepia'] { … }` 原样迁入 |

迁移原则：**逐字搬运，不改值**；每个分节头注明「迁自 <文件>@ac81857（2026-09-10）」。当前 `components.css` 里与原型同名但取值属于旧原型的规则（如 `.nav-dot`、`.tag-chip` 的 padding、`.todo-item` 的 display、`.settings-body select` 的配色）一律不迁，由新原型的生成层接管。

---

## 7. 令牌与主题

### 7.1 令牌
- 尺度令牌、行高、圆角阶、阴影阶、层级、焦点环、`--mm-*` 全部随 tokens.css 进入；**不**额外把组件裸数值改为令牌引用（原型怎么写就怎么来）。
- 动效令牌以原型为准：`--dur-fast 120ms`、`--dur-base 180ms`、`--dur-slow 280ms`、`--ease-out cubic-bezier(.22,.61,.36,1)`、`--ease-spring cubic-bezier(.34,1.56,.64,1)`。当前应用的 `--ease-out` 值与原型不同，按原型覆盖。项目扩展令牌见第 6 节。
- `motion.css` 引用的 `--dur-instant/--lift/--stagger/--ease-in-out` 由 extensions.css 提供，因此 motion.css 本阶段零改动。

### 7.2 主题清单 `src/constants/themes.ts`
- 顺序与原型 `THEMES` 逐字一致：`typewriter`（打字机，色点 `#ece7db #c62828 #1a1a1a #2e7d32`）第一，随后 `dark light cupcake bumblebee emerald business neon retro romance halloween fantasy oled luxury dracula print autumn businessgray psychedelic lemon night coffee winter abyss aqua latte dim aurora pastel sunset wireframe`，最后追加项目自建 `sepia`（棕褐），共 32 套。
- `DEFAULT_THEME = 'typewriter'`。
- 文件头注释更新数据来源为 `docs/app.js` 的 THEMES。

### 7.3 主题写入 `src/composables/useTheme.ts`
- `writeToDom` 改为**任何主题都设置** `data-theme`（含 `dark`），与原型 `document.documentElement.dataset.theme = t.id` 一致；`dark` 没有对应 CSS 块，落到 `:root` 基础令牌。
- 其余逻辑（localStorage 抢先上色、settings 权威校正）不变。

### 7.4 入口 html
九个入口（`index panel pinned reminder hotzone island launcher editor mindmap`）的 `<html>` 加 `data-theme="typewriter"`，首帧即打字机；随后 `applyCachedTheme` 以缓存 / 后端值校正。

### 7.5 后端默认值与 v5 迁移（`src-tauri/src/data/mod.rs`、`domain/models.rs`）
- `settings` 表初始化插入与 `Settings::default()` 的 `theme` 改为 `typewriter`。
- 新增 `with_v5`：`UPDATE settings SET value='typewriter' WHERE key='theme' AND value='dark'`；`migrate()` 中 `version < 5` 时执行并把 `user_version` 更新为 5。
- 单元测试（先写失败测试再实现）：① `dark` 被迁移为 `typewriter`；② 其他主题值不动；③ 重复执行幂等；④ 现有 v4 测试中断言 `theme == "dark"` 的用例按新语义调整。
- 前端 `VALID_KEYS` 已含 `typewriter`，老缓存值若为 `dark` 会被后端返回的 `typewriter` 校正。

### 7.6 与扩展的交互
- 打字机主题对 `.glass`、`.context-menu`、`.prio-group`、`.repeat-menu` 直接写 `backdrop-filter: none`，优先级高于 `glass.css` 的档位令牌，因此打字机下三档质感自然失效，符合「纸面终端无毛玻璃」的定义；其他主题不受影响。
- `data-acrylic='off'` 在打字机下把 `--glass-bg` 降级为 `--bg-deep`（`#ddd6c4`），视觉合理，不需特殊处理。

---

## 8. 动效模块 `src/motion/`

新增目录，独立于样式层；只依赖 `animejs` 与 `service/logger`。

### 8.1 文件与接口

| 文件 | 导出 | 说明 |
|---|---|---|
| `tokens.ts` | `readMotionTokens(): MotionTokens` | 用 `getComputedStyle(document.documentElement)` 读取 `--dur-fast/base/slow`、`--stagger`、`--ease-out/--ease-spring`；解析 `ms` 为数字、`cubic-bezier(...)` 为 animejs 的 `cubicBezier(...)`。reduced-motion 下（`matchMedia('(prefers-reduced-motion: reduce)')`）全部时长返回 0，预设据此直接落到终态、不创建动画 |
| `presets.ts` | `enter(el, opts)`、`exit(el, opts)`、`staggerIn(els, opts)`、`pop(el)`、`crossfade(outEl, inEl)` | 均返回 `Promise<void>`；`opts` 含 `axis: 'x' 或 'y'`、`distance: number`、`scale?: number`；完成或取消后一律清除内联 `transform/opacity` |
| `directive.ts` | `vStaggerList: Directive<HTMLElement, unknown>` | 见 8.4 |
| `index.ts` | 统一出口 | 调用方不接触 animejs API |

**取消语义**：每个元素上通过 `WeakMap<Element, JSAnimation>` 记录进行中的动画；新预设作用于同一元素时先 `cancel()` 旧动画并清内联样式，再开始新动画。

### 8.2 预设参数（与原型 app.js 逐字对应）

| 预设 | 起点 → 终点 | 时长 | 缓动 | 原型出处 |
|---|---|---|---|---|
| `enter` | 位移 30px（沿 axis，方向由调用方给）、opacity 0、scale .97 → 0 / 1 / 1 | `--dur-slow` 280ms | `outBack(1.6)` | `showPanel` |
| `exit` | 0 / 1 → 位移 24px、opacity 0 | `--dur-base` 180ms | `inQuad` | `hidePanel`（power2.in） |
| `staggerIn` | 位移 8px、opacity 0 → 0 / 1，逐项延迟 `--stagger` 30ms | `--dur-base` 180ms | `--ease-out` | 微交互（不改变视觉语言） |
| `pop` | scale .95 → 1 | `--dur-fast` 120ms | `--ease-spring` | 微交互 |
| `crossfade` | 旧元素 opacity 1→0（`--dur-fast`），新元素 0→1（`--dur-base`） | — | `--ease-out` | 微交互 |

### 8.3 硬约束（写入模块头注释）
1. 带 `backdrop-filter` 的 `.glass` 只允许在入场 / 收起这类瞬时动效里加 transform，静止态必须无 transform。原型入场含 scale .97，先按原型实现；实机若在 WebView2 下出现毛玻璃错位则去掉 scale 只留位移，并把结论补记到本文档「实机结论」小节。
2. 不用 CSS `animation` + `fill-mode: both` 做入场；入场一律 JS 驱动并由 `panelShown` 事件重新触发。
3. 只动 transform 与 opacity。
4. 每个视图同一时刻最多 1–2 个动效元素；退出时长为入场的 60–70%；动效可被后续操作打断。

### 8.4 列表错落指令 `v-stagger-list`
- 用法：`<ul v-stagger-list>`，无参数。
- 行为：`mounted` 与宿主组件每次 `updated` 后检查容器的**直接子元素**，只对**首次出现**的子元素（以 WeakSet 记录）取前 12 个执行 `staggerIn`，已出现过的不动、超出 12 个的立即显示。keyed `v-for` 复用的 DOM 节点不算新出现，因此删除、重排、搜索过滤中保留下来的项不会重播；异步加载后新填充的项会自然入场。
- 同一容器再次触发时先取消进行中的动画；动画期间给参与元素加 `.is-animating`，完成 / 取消后移除并清内联样式。
- reduced-motion 下不做任何 DOM 写入。
- 不使用 `<TransitionGroup>`：它依赖 CSS 类与 fill 态，违反硬约束 2。

### 8.5 面板迁移 `src/windows/Panel/PanelApp.vue`
- `hide()`：`await exit(panel, { axis, distance: 24 方向化 })` 后再 `panelHide`。
- `playEnter()`：`void enter(panel, { axis, distance: 30 方向化, scale: 0.97 })`。
- 位移轴与方向仍按 `panel_position` 推导（`motionAxis`/`motionDistance` 保留）。
- 删除 `import gsap`；`pnpm remove gsap`；全仓 `grep -r gsap src` 必须为空。

### 8.6 错落接入点（五处模板，覆盖六个列表）

| 位置 | 容器 |
|---|---|
| `windows/Main/NotesView.vue` | 包住 NoteCard 的列表容器 `<div v-stagger-list>` |
| `windows/Main/ClipsView.vue` | `<ul v-stagger-list>` |
| `windows/Main/DayView.vue` | 日期详情列表容器 `<div v-stagger-list>` |
| `windows/Panel/ClipPage.vue` | `<ul v-stagger-list class="clip-list">` |
| `components/card/TodoTree.vue` | 顶层 `<ul v-stagger-list class="todo-list">`——同时覆盖主窗口待办页与面板待办页；只对顶层项与分区标题错落，子级 `.todo-children` 不接 |

启动台结果列表不接（键盘即时响应优先，原型亦无此动效）。

---

## 9. 验证清单

每一步都要拿到命令输出后才能声称完成。

1. `pnpm sync:styles --check` 通过；再执行一次 `pnpm sync:styles` 后 `git status` 无变化（幂等）。
2. 令牌唯一性检查通过（脚本内置）。
3. 选择器集合复核：新原型选择器在 `src/styles` 中缺失数为 0（排除被过滤的演示项）。
4. `pnpm typecheck` 零错误；`cargo check --manifest-path src-tauri/Cargo.toml` 零错误；`cargo test --manifest-path src-tauri/Cargo.toml` 全部通过（含 v5 三个新用例）。
5. `grep -r gsap src` 为空；`package.json` 无 `gsap`；`pnpm-lock.yaml` 已更新。
6. 实机 `pnpm tauri:dev`：主窗口、面板、置顶、提醒、感应区、灵动岛、启动台、独立编辑、思维导图九个窗口，在**打字机**与**深色**两套主题下逐一打开，与浏览器中的 `docs/index.html` 并排比对。阶段一验收标准是「令牌与主题一致、无破损」，页面结构差异属于后续阶段。若新组件样式导致破损，只允许在 `extensions.css` 或 `window-fit.css` 加最小覆盖，并注明所属阶段的 TODO。
7. 面板入场 / 收起：打字机与深色下各触发 5 次，无残影、无停在透明态；窗口隐藏再显示后入场动画重新播放。
8. 列表错落：六处列表首次进入与搜索词变化时逐项入场；动画结束后卡片 hover 抬升正常（内联样式已清）。
9. 系统开启「减少动态效果」后重启应用：无动效但内容完整可见。
10. 主题切换：设置页下拉顺序与原型一致，切到每一套主题均无控制台错误；新建数据库默认打字机；旧库（`theme=dark`）启动后变为打字机且 `user_version=5`。

---

## 10. 提交批次（中文提交信息）

1. `feat(styles): 原型样式同步脚本与四层生成样式，自建样式收拢到 extensions.css`
2. `feat(theme): 打字机默认主题、主题清单对齐原型、入口首帧主题与 v5 迁移`
3. `feat(motion): animejs 动效模块替换 gsap，面板入场收起对齐原型`
4. `feat(motion): 列表错落入场指令与六处接入`
5. `docs(design): 阶段一实机验收记录`（含实机结论：scale 是否保留、破损补丁清单）

每批提交前必须通过第 9 节对应检查项。

---

## 11. 风险与回退

| 风险 | 应对 |
|---|---|
| 新 components.css 的 `#panel`/`.nav-dot`/`.archive-*` 等规则与当前 Vue 标记不完全匹配，出现结构性错位 | 阶段一只保证「不破损」；错位若源于结构差异，记入对应阶段的待办，不在本阶段改标记 |
| 质量层 `:focus-visible` 的 box-shadow 焦点环与应用自建焦点样式叠加 | 验收逐窗口 Tab 巡检；重复者删自建条目 |
| `* { user-select: none }` 让新出现的可编辑区不可选中 | extensions.css 的恢复规则覆盖 `input/textarea/[contenteditable]/.ProseMirror/.cm-editor`，验收补测启动台输入框 |
| animejs 内联样式残留导致 CSS hover 失效 | 预设完成 / 取消统一清内联样式，验收第 8 项 |
| 打字机等宽字体在 Windows 回退到 Consolas，中文回退到微软雅黑，字宽混排 | 原型即如此，按原型；不自行改字体栈 |
| v5 迁移把主动选深色的用户切到打字机 | 已接受（D8），重选一次即可 |

回退：四个生成文件可由脚本随时重生；主题与迁移各自独立提交，可单独 revert；gsap 迁移独立提交。

---

## 12. 实机结论（实施后填写）

- 面板入场 scale .97 在 WebView2 下是否保留：待填。
- 破损补丁清单（文件 / 选择器 / 所属阶段）：待填。
