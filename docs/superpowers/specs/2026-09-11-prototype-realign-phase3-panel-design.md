# 原型对齐 · 阶段三「呼出面板」设计

**日期**：2026-09-11
**前置**：阶段一 `2026-09-10-prototype-realign-phase1-foundation-design.md`（样式分层 / 动效模块）、阶段二 `2026-09-10-prototype-realign-phase2-main-window-design.md`（D10–D14）
**原型**：`docs/index.html` `#panel`（81–129 行）、`docs/app.js` 面板相关函数、`docs/styles.css` 316–384 行；生成层 `src/styles/components.css` 已含全部面板规则

## 1. 范围

把呼出面板（`src/windows/Panel/`）对齐原型：矢量三态圆点导航、Zen 专注模式、编辑器底栏与暂存文案、笔记「回显到面板」编辑态（兑现阶段二 D13）、粘贴板页卡片（含来源应用）、待办页提示行、失焦收起触发源、Esc 链与切页副作用。

**不在本阶段**：面板内弹窗（标签管理 / 粘贴板编辑 / 待办编辑）的结构对齐、删除确认浮层、优先级阶梯菜单、重复提醒菜单、灵动岛淡出联动（阶段四）；编辑器实现（保留 ProseMirror）。

## 2. 决策记录（本阶段新增，均已确认）

| # | 问题 | 采用 | 理由 |
|---|---|---|---|
| D15 | Zen 窗口形态 | 后端新增 `panel_set_zen`，把现有面板窗口放大到光标所在屏工作区，退出还原；前端 `#panel.zen-mode` | 一个窗口、编辑状态连续；生成层样式已就绪 |
| D16 | 回显编辑态 | 完全按原型：先落库当前草稿 → 载入笔记进编辑态（按钮「保存修改 ✓」，编辑态不自动暂存）→ 收面板即丢弃未保存修改并恢复草稿 → 主窗口隐藏 | 原型 `openNoteInPanel` / `hidePanel` 语义 |
| D17 | 失焦收起触发源 | 窗口 `blur` 与鼠标 `mouseleave` 并存：任一发生即计时；`mouseenter` 或 `focus` 取消 | 兼顾原型行为与多窗口真实场景 |
| D18 | 编辑器 Enter | 保持 Enter 换行、Mod+Enter 归档 | 多行 Markdown 更自然；与原型差异记入 §3 |
| D19 | 感应区悬停阈值 | 保持 3 秒（含进度指示器） | 既有需求指定 |
| D20 | 粘贴板来源应用 | 阶段三补齐：采集时读前台窗口进程名，`clipboard_entries` 加 `source_app` 列（v6 迁移），面板与主窗口一起显示 | 用户要求 |
| D21 | 面板内弹窗 | 留阶段四；本阶段只改副标题文案 | 弹窗属小窗口范围 |

## 3. 与原型的差异处理表

| # | 项 | 原型 | 现状 | 做法 |
|---|---|---|---|---|
| 1 | 圆点导航结构 | `.nav-dots[role=tablist]` > `button.nav-dot.dot-note/.dot-clip/.dot-todo[role=tab][aria-selected][aria-label][title="笔记 (⌃1)"]` | `span.nav-dot` 直接放 `.panel-nav`，emoji 文本，无 ARIA | 就地：改 button + 包裹 + ARIA；注册表 `dot: '🔴'` 改为 `dotClass: 'dot-note'` |
| 2 | 圆点绘制 | `::before` 10px 矢量圆，`rgb(var(--dot-c))`，hover/active 走 opacity + scale + 光环 | 桥接层屏蔽 `::before`，动效层对本体 scale | 删 `extensions.css` §5 圆点两段、删 `motion.css` `.nav-dot` 段；生成层规则即刻生效 |
| 3 | 导航条右侧 | `#zenToggle.btn.ghost.tiny`「🧘 Zen」+ `.panel-hint` | 只有 `.panel-hint` | 就地：加 Zen 入口，显隐 = 面板可见 ∧ 笔记页 ∧ 编辑态 ∧ 非 Zen |
| 4 | Zen 布局 | `#panel.zen-mode` 铺满、隐藏导航 / 标签 / 暂存态、编辑器 18px、底栏只剩「退出 Zen」+ 保存 | 无 | D15；见 §4.3 |
| 5 | 编辑器 | contenteditable + 正则渲染 | ProseMirror | 保留（差异接受） |
| 6 | Enter | Enter 归档 / Shift+Enter 换行 | Enter 换行 / Mod+Enter 归档 | D18 保留（差异接受） |
| 7 | 暂存文案 | 初始「已暂存」；输入中「输入中…」+ `.saving`；500ms 后「已暂存 SQLite」 | 「未保存 / 暂存中… / 已暂存」 | 就地对齐三段文案与 `.saving` 时机 |
| 8 | 标签预览 +N | 冒泡到 `tagPreview` 打开标签管理 | `TagList` +N 就地展开 | `TagList` 增 prop `moreAction: 'expand' \| 'open'`，面板传 `open` |
| 9 | 标签管理副标题 | 草稿态「当前正在编写的念头（未归档）的标签」/ 编辑态「当前笔记的标签」 | 固定 | 就地按编辑态切换 |
| 10 | 归档后 | 清空 → 250ms 后收面板；toast「先写点什么吧」「念头已归档 ✔」 | 不收面板；文案不同 | 就地：NotePage emit `archived`，PanelApp 250ms 后 `hide()`；文案对齐 |
| 11 | 呼出时聚焦 | 显示 100ms 后 `editor.focus()` | 无 | 就地：`panelShown` 且当前为笔记页时调 NotePage `focus()` |
| 12 | 回显编辑态 | `editingNoteId`、按钮「保存修改 ✓」、toast「内容已回显，修改后点击「保存修改」」、收面板丢弃 | 无；主窗口 ✏️ 用 `NoteEditModal` | D16；见 §4.2 |
| 13 | 入场 / 收起动效 | 同现状参数 | 已对齐 | Zen 态跳过位移动画、暂停高度上报 |
| 14 | 失焦触发 | `mouseleave` 计时 / `mouseenter` 取消 | `blur` / `focus` | D17 并存 |
| 15 | Esc 链 | 先关浮层与删除确认再收面板 | `ConfirmPopover` 不拦 Esc | 就地：各页暴露 `dismissOverlays(): boolean`，PanelApp Esc 先调它，返回 true 则不收面板 |
| 16 | 切页副作用 | `clearPendingDelete()` | 各页确认态不清 | 就地：`watch(activeId)` 调各页 `dismissOverlays()` |
| 17 | 粘贴板卡片头 | `.clip-time`（📌 前缀）+ `.clip-from`「🌐 应用名」+ `.clip-type` | 仅时间；类型徽章仅归档形态 | 就地：面板卡片也渲染类型徽章、📌 前缀、来源应用（D20）；`ClipArchiveCard` 同步加来源应用 |
| 18 | 粘贴板卡片图标 | 内联 SVG | emoji | 改用 `Icon.vue`（阶段二已有 close / pin / edit / paste / link 五枚，缺的补） |
| 19 | 双击粘贴 | 置顶 + toast | 置顶 + 真粘贴 | 保留（能力超集） |
| 20 | 待办页提示行 | `.todo-panel-hint` | 无 | 就地加一行 |
| 21 | 待办 🔁 重复菜单 | 面板内菜单 | 事件未接 | 接到独立编辑窗（focus `remind`），菜单本身阶段四 |
| 22 | 面板高度 | 内容撑开 | 上报 90–600 | 保留；注释「120」改为 90 |
| 23 | `data-window='panel'` | — | `window-fit.css` 面板整页化规则以它为前缀但无人设置 | `panel.ts` 设置 `dataset.window = 'panel'`，实机核对整页化是否符合预期；若观感倒退则删除该组规则 |
| 24 | 感应区阈值 | 100ms | 3000ms | D19 保留 |

## 4. 设计

### 4.1 圆点导航（`PanelApp.vue`、`panel-plugins/index.ts`）

- `PanelPlugin` 接口：`dot: string` → `dotClass: 'dot-note' | 'dot-clip' | 'dot-todo' | string`；注册表内置三项按原型类名。新插件若无对应令牌，生成层 `.nav-dot::before` 的 `rgb(var(--dot-c))` 会因变量缺失而失效，故 `extensions.css` §3 补一条 `.nav-dots { --dot-c: var(--accent-rgb) }` 兜底：自定义属性向下继承，`.dot-*` 在元素自身上声明的值优先于父级继承值，不受加载顺序影响。
- 模板：

```vue
<div class="panel-nav">
  <div class="nav-dots" role="tablist" aria-label="捕获模式切换">
    <button v-for="(plugin, index) in plugins" :key="plugin.id" type="button"
      class="nav-dot" :class="[plugin.dotClass, { active: activeId === plugin.id }]"
      :data-mode="plugin.id" role="tab" :aria-selected="activeId === plugin.id"
      :aria-label="`${plugin.label}模式`" :title="hotkeyTitle(plugin, index)"
      @click="navigateTo(plugin.id)" />
  </div>
  <button v-if="zenAvailable" id="zenToggle" type="button" class="btn ghost tiny" title="Zen 专注模式：全屏沉浸编辑（Esc 退出）" @click="setZen(true)"><span class="ix">🧘</span> Zen</button>
  <span class="panel-hint">Esc 收起</span>
</div>
```

### 4.2 回显编辑态（前后端）

**后端**

- `state.rs`：`pending_panel_page: Option<String>` 改名为 `pending_panel_intent: Option<String>`，存 JSON `{"page":"note","noteId":"…"}` / `{"page":"todo"}`；`take_pending_panel_intent()`。
- `windows.rs`：`panel_show_page(app, page)` 改为写 intent `{"page":page}`；新增 `panel_open_note(app, note_id)`：写 intent `{"page":"note","noteId":id}` → `hide_main(app)` → `panel_show(app)`。
- `ipc.rs`：`panel_take_page` 改为 `panel_take_intent` 返回 `Option<String>`；新增 `pub async fn panel_open_note(app, note_id)`（走 `panel_show` 不建窗，async 只为不阻塞主线程 hide_main+show 序列，与 `launcher_show` 同款）；新增 `note_get(id) -> Option<Note>`（`data/notes.rs` 暴露 `get_note`，复用内部 `note()`）。
- `main.rs` 注册；`hotzone_watcher.rs` 灵动岛点击改调 `panel_show_page(&app, "todo")` 不变（内部已改 intent）。
- 事件：不新增。面板可见时 `panel_show` 仍 emit `PANEL_SHOWN`，前端每次 take intent，一条路径覆盖隐藏 / 可见两种情况；`PANEL_NAVIGATE` 保留给面板已可见时的即时切页。

**前端**

- `tauri.ts`：`panelTakeIntent(): Promise<PanelIntent | null>`（解析 JSON）、`panelOpenNote(id)`、`panelSetZen(on)`；`notes.get(id)`。
- `PanelApp.vue`：`panelShown` → `takeIntent()` → `navigateTo(intent.page)`；若 `intent.noteId` → 调笔记页实例 `loadNote(noteId)`。插件实例引用：`<component :ref="(el) => pageRefs[plugin.id] = el">`，类型 `Record<string, PanelPageExpose | null>`；`PanelPageExpose = { focus?(): void; dismissOverlays?(): boolean; loadNote?(id: string): Promise<void>; onPanelHide?(): void }` 定义在 `panel-plugins/index.ts`。
- `NotePage.vue`：
  - 状态 `editingNoteId: string | null`、`editingSnapshot: Note | null`。
  - `loadNote(id)`：若当前草稿非空先 `await persistDraft()`；`api.notes.get(id)`；`editingNoteId = id`；`content = note.content`、`tags = [...note.tags]`；关闭自动暂存（`watch` 回调里 `if (editingNoteId.value) return`）；按钮文案「保存修改 ✓」；toast「内容已回显，修改后点击「保存修改」」；`focus()`。
  - `archive()`：编辑态走 `api.notes.save({ id: editingNoteId, …, draft: false })`，toast「修改已保存 ✔」，退出编辑态并 `await loadDraft()` 恢复草稿；新建态 toast「念头已归档 ✔」。两者成功后 `emit('archived')`。
  - `onPanelHide()`：编辑态 → 丢弃（`editingNoteId = null`，`await loadDraft()`），按钮文案复原。PanelApp 在 `hide()` 开始时调当前所有页的 `onPanelHide`。
  - `defineExpose({ focus, dismissOverlays, loadNote, onPanelHide })`。
- `NotesView.vue`、`DayView.vue`：文本笔记 ✏️ → `api.windows.panelOpenNote(note.id)`；导图仍 `mindmapOpen`。`NoteEditModal.vue` 删除（两处调用都改掉后无引用）。

### 4.3 Zen（D15）

**后端** `windows.rs`：

- `static PANEL_ZEN: AtomicBool`。
- `panel_set_zen(app, on)`：`on` → 记录 `PANEL_ZEN=true`，取光标屏 `WorkArea`，`set_position(work.left, work.top)`、`set_size(work.width, work.height)`（物理像素）；`off` → `PANEL_ZEN=false`，按 `PANEL_LOGICAL_HEIGHT` 调 `place_panel`。
- `panel_resize`：`PANEL_ZEN` 为真时只更新 `PANEL_LOGICAL_HEIGHT` 不动窗口。
- `panel_hide`：若 `PANEL_ZEN` 为真先 `panel_set_zen(false)`。
- `ipc.rs` 新增 `panel_set_zen(on: bool)`；`main.rs` 注册。

**前端**

- `PanelApp.vue`：`zen = ref(false)`；`setZen(on)`：`zen.value = on; await api.windows.panelSetZen(on); if (on) nextTick(() => notePage.focus())`；根元素 `:class="{ 'zen-mode': zen }"`；Zen 时 `reportHeight` 直接 return；`hide()` 与 Esc 首位先 `setZen(false)`（Esc 在 Zen 态只退 Zen 不收面板，与原型 Esc 链一致）；`zenAvailable = computed(() => visible && activeId === 'note' && notePage?.editingNoteId)`——通过 `pageRefs.note?.isEditing?.value`，`NotePage` expose 一个 `isEditing: Ref<boolean>`。
- `NotePage.vue` 底栏：`.editor-actions` 里加 `<button id="zenExit" type="button" class="btn ghost" @click="emit('zen-exit')"><span class="ix">🧘</span> 退出 Zen</button>`，显隐完全交给生成层 `#zenExit{display:none}` / `#panel.zen-mode #zenExit{display:inline-block}`，不传 prop；PanelApp 监听 `@zen-exit="setZen(false)"`。
- `window-fit.css` 第 13 行把 `#panel` 设为 `position: static; width: 100%`，Zen 态生成层的 `top/left/width:100vw/height:100vh` 被覆盖但无害（窗口本身已放大），补一条 `#panel.zen-mode { height: 100vh }` 到 `window-fit.css` 保证纵向撑满。

### 4.4 失焦收起（D17）

`PanelApp.vue`：现有 `scheduleCollapse` / `clearCollapseTimer` 不变；触发源增加根元素 `@mouseleave="scheduleCollapse"`、`@mouseenter="clearCollapseTimer"`；Zen 态与 `modalDepth > 0` / `externalEditorOpen` 时忽略 `mouseleave`（Zen 铺满屏幕，鼠标离开即离开屏幕，不应收起）。

### 4.5 粘贴板来源应用（D20）

**后端**

- `Cargo.toml` `windows-sys` 加特性 `Win32_UI_WindowsAndMessaging`、`Win32_System_Threading`。
- `platform.rs` 新增 `pub fn foreground_app_name() -> Option<String>`（Windows：`GetForegroundWindow` → `GetWindowThreadProcessId` → `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` → `QueryFullProcessImageNameW` → 取文件名去 `.exe`；非 Windows 返回 None）。自身进程返回 None（回声抑制之外再防一手）。
- `data/mod.rs`：v6 迁移 `add_column("clipboard_entries", "source_app", "TEXT")` + `user_version = 6`；单测一条。
- `data/clipboard.rs`：`Capture` 加 `source_app: Option<String>`；`ROW_COLUMNS` 与 `row_entry`、INSERT 加列；`ClipboardEntry` 加 `#[serde(skip_serializing_if = "Option::is_none")] source_app: Option<String>`。
- `clipboard_watcher.rs`：`poll_once` 采集到新内容时调 `foreground_app_name()` 写入；手动捕获（`clipboard_capture`）同样。

**前端**

- `domain.ts` `ClipboardEntry.source_app?: string | null`。
- `ClipCard.vue`（面板）与 `ClipArchiveCard.vue`：`.clip-head` 内 `<span v-if="entry.source_app" class="clip-from" title="来源应用"><span class="ix">🌐</span> {{ entry.source_app }}</span>`。无值不渲染（不显示「未知应用」占位）。

### 4.6 粘贴板卡片（面板 `ClipCard.vue`）

删除 `archive` prop（无人再传）；结构对齐原型 `renderClips`：`.card-close`（`Icon close`）、`.clip-head`（`.clip-time` 带 📌 前缀 → `.clip-from` → `ClipTypeBadge`）、`.clip-text`、`.clip-ops`（paste / link / edit / pin，全部 `Icon`）。`Icon.vue` 若缺 `paste` 图标，按原型 `ICON_PASTE` 补一枚。

### 4.7 其他小项

- `TagList.vue`：prop `moreAction?: 'expand' | 'open'`（默认 `expand`）；`open` 时 +N 与 chips 不 `stop`，由父级 `.tag-preview` 的 click 打开管理弹窗。
- `TagManagerModal` 调用处副标题：`editingNoteId ? '当前笔记的标签' : '当前正在编写的念头（未归档）的标签'`。
- `TodoPage.vue`：加 `.todo-panel-hint`；`edit-repeat` → 打开独立编辑窗 `focus: 'remind'`。
- `ClipPage.vue` / `TodoPage.vue` expose `dismissOverlays()`：有待确认删除则取消并返回 true。
- `panel.ts`：`document.documentElement.dataset.window = 'panel'`。

## 5. 测试

- Rust：v6 迁移单测（加列幂等、旧行 `source_app` 为 NULL）；`foreground_app_name` 不测（依赖前台）；`panel_set_zen` 几何函数若抽为纯函数则测。
- 前端 `node:test`：无新纯函数；`panel-plugins/index.ts` 的 `resolvePlugins` 已有覆盖不变。
- `pnpm typecheck`、`pnpm test`、`cargo test`、`pnpm sync:styles --check`、`vite build`。

## 6. 实机验收（打字机 + 深色）

1. 圆点：三色矢量圆、hover / active 光环、⌃1/2/3、Tab 焦点环。
2. 笔记页：暂存三段文案；标签预览三形态，+N 打开管理弹窗；归档后 250ms 收面板；呼出后光标在编辑器。
3. 回显：主窗口 ✏️ → 主窗口隐藏、面板显示、内容与标签回显、按钮「保存修改 ✓」、Zen 按钮出现；保存修改 → 主窗口列表更新、面板收起、草稿恢复；不保存直接 Esc → 修改丢弃、草稿恢复。
4. Zen：进入铺满工作区、导航 / 标签 / 暂存态隐藏、18px；Esc 退出还原尺寸与位置；Zen 中鼠标移出不收起。
5. 失焦：鼠标移出 3 秒收起、移回取消；切到别的窗口 3 秒收起；「立即 / 固定」两档。
6. Esc 链：有删除确认时 Esc 只关确认。
7. 粘贴板页：来源应用显示（从记事本复制一段 → 显示「Notepad」）、类型徽章、📌 前缀、SVG 图标、双击粘贴。
8. 待办页：提示行；🔁 打开编辑窗。
9. 面板整页化弹窗（`data-window='panel'` 生效后）观感。

## 7. 提交批次（中文）

1. `feat(ipc): 面板 intent 槽位、panel_open_note / note_get / panel_set_zen 命令`
2. `feat(clipboard): 采集来源应用（v6 迁移 source_app）`
3. `feat(panel): 矢量圆点导航与 ARIA，删除 emoji 桥接`
4. `feat(panel): 笔记回显编辑态 + Zen 专注模式，主窗口 ✏️ 改为回显到面板`
5. `feat(panel): 编辑器底栏文案 / 标签预览 / 归档收起 / 呼出聚焦`
6. `feat(panel): 粘贴板卡片对齐原型（来源应用 / 类型徽章 / SVG 图标）`
7. `feat(panel): 失焦触发源并存、Esc 链、切页清确认、待办提示行`
8. `docs(design): 阶段三实机验收记录`

## 8. 风险

| 风险 | 应对 |
|---|---|
| `panel_set_zen` 放大后 `resizable(false)` 窗口在多屏切换时位置错乱 | 只取进入 Zen 时的光标屏；退出按当前唤出位置重摆 |
| 编辑态与草稿互相覆盖 | 回显前先落库草稿；编辑态关闭自动暂存；退出编辑态重新 `loadDraft` |
| `QueryFullProcessImageNameW` 对提权进程失败 | 返回 None，不渲染来源 |
| `data-window='panel'` 首次生效可能改变弹窗观感 | 验收第 9 项决定去留 |
| 删除 `NoteEditModal` 后 `DayView` 编辑粘贴板 / 待办不受影响 | 只替换笔记分支 |

## 9. 实机结论（2026-09-12 回填，详见 `../plans/2026-09-11-prototype-realign-phase3-acceptance.md`）

- Zen 放大 / 还原：**可用**。单屏 2560×1600@150% 下铺满工作区 0,0 2560×1528（不遮任务栏），Esc / 「退出 Zen」还原到顶部居中、逻辑高度不变；快捷键在 Zen 中直接收起再呼出时由 `PANEL_SHOWN wasHidden` 重同步，不残留 Zen 布局。多屏未测。
- 来源应用采集命中率：记事本 Ctrl+C 命中「Notepad」；本应用内粘贴��生的条目为空（自身进程按设计不记）。VS Code / 浏览器 / 提权进程未测。
- 补丁清单：本阶段无 CSS 补丁；代码修复全部来自审查环节并随任务提交（`0523bf1` Zen 态重摆、`4f7fcd1` wasHidden 重同步与退出 Zen 回滚、`5f76c52` 暂存状态复位、`0e05f91` 日期详情监听 / 空草稿落库 / 意图解析抽纯函数）。`data-window='panel'` 生效后面板内弹窗观感正常，`window-fit.css` 该组规则保留。
- 遗留：主窗口一次进入外部 ShowWindow 无效的隐藏态（run10，重启后消失，未定位）；`ConfirmPopover` 首张卡片上方被裁切（阶段四）；未验证项与产品确认项见验收记录 §5。
