# 原型对齐 · 阶段五「思维导图窗口」实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把已完备的导图窗口（`simple-mind-map` + naive-ui，70 个文件）的**外壳**对齐到 `docs/index.html` 的原型形态（顶栏双工具岛 + 文件名岛、单抽屉壳、图标/公式模态、底栏重排、右键菜单挂 body），并给其余 30 套主题接入 `--mm-*` 令牌。

**Architecture:** 形态照原型、能力用我们的（spec D47）：组件改用原型类名，让**生成层** `src/styles/components.css:652-1484`（由 `docs/styles.css` 生成，含 104 处 `--mm-*`）直接接管样式；自有层 `mindmap.css` 收敛为「真实窗口适配 + 项目扩展」。原型的 16 处演示桩（格式刷/关联线只 toast、小地图假图、搜索不定位、外框下拉取值被丢弃、AI 插写死节点……）**一律不照抄**，保留我们的真实现。

**Tech Stack:** Vue 3.5 `<script setup lang="ts">`、TypeScript 5.9、naive-ui 2.45、simple-mind-map 0.14.0-fix.3、Tauri 2（Rust，本阶段不动后端）

**Spec:** `docs/superpowers/specs/2026-09-14-prototype-realign-phase5-mindmap-design.md`（D41–D49）
**勘察存档（行号依据）:** `.superpowers/sdd/recon-phase5-prototype.md`、`.superpowers/sdd/recon-phase5-current.md`

## Global Constraints

- 回答与提交信息用中文；提交格式 `<type>(scope): <中文描述>`，scope 用 `mindmap`。每完成一个 Task 立即提交，不攒。
- 前端提交前必须通过 `pnpm typecheck`（`vue-tsc --noEmit`）与 `pnpm format:check`；涉及 Rust 时 `cargo fmt --check` + `cargo test`。
- 全项目禁止裸 `console.*`，一律 `logger.info/error('mindmap', ...)`；关键节点必须有日志（改名提交、模态开关、主题令牌生成、右键菜单挂载）。
- **禁止手改生成层** `src/styles/{tokens,base,components,themes}.css`——它们由 `scripts/sync-prototype-styles.mjs` 从 `docs/styles.css` 生成，`pnpm sync:styles --check` 必须通过。\n  例外：spec D46 要求给 30 套主题补 `--mm-*`，但那要改 `docs/styles.css` 的对应主题块**再跑 `pnpm sync:styles`**，不是直接改 `themes.css`。
- 每个 `.vue` / `.ts` 文件头部有说明职责的注释块（沿用项目既有密度）。
- naive-ui **只在导图窗口引入**，不得被 `panel.ts` / `main.ts` / `hotzone.ts` 间接 import。
- 本阶段**不动后端**（无 Rust 改动），也不动 `export_*` 相关命令。
- 仓库 `core.autocrlf=true`：遇 `format:check` 报行尾时，只对改动文件跑 `prettier --write`，不要改 `.prettierrc`。

---

## File Structure

| 路径 | 职责 | 动作 |
|---|---|---|
| `src/utils/mindmapName.ts` | 导图名规范化纯函数（`normalizeMapName`） | 新建 |
| `src/utils/mindmapName.test.ts` | 上者的 `node:test` 单测 | 新建 |
| `src/windows/MindMap/chrome/MmFilenameIsland.vue` | 文件名岛（显示 / 编辑 / 提交改名） | 新建 |
| `src/windows/MindMap/chrome/Toolbar.vue` | 顶部工具条 → 原型三岛结构 | 改造 |
| `src/windows/MindMap/popups/MmModal.vue` | 通用模态壳（图标 / 公式复用） | 新建 |
| `src/windows/MindMap/popups/NodeIconModal.vue` | 图标模态（由 `IconSidebar` 内容改造） | 新建 |
| `src/windows/MindMap/popups/NodeFormulaModal.vue` | 公式模态（由 `FormulaSidebar` 内容改造） | 新建 |
| `src/windows/MindMap/chrome/MmBottombar.vue` | 底部控制栏（`Count` + `NavigatorToolbar` 合并） | 新建 |
| `src/windows/MindMap/sidebars/SidebarShell.vue` | 10 实例 → 单抽屉壳 | 改造 |
| `src/windows/MindMap/popups/ContextMenu.vue` | 改挂 body + 令牌改 `--mm-*` | 改造 |
| `src/windows/MindMap/core/naiveTheme.ts` | 明暗基底按 `--scheme` 选；两处硬编码改令牌 | 改造 |
| `docs/styles.css` | 30 套主题块各补 13 个 `--mm-*` | 改造（再 `pnpm sync:styles`） |
| `src/windows/MindMap/sidebars/NoteSidebar.vue` | 无入口死码 | 删除 |
| `src/windows/MindMap/core/store.ts` | `SidebarName` 去掉 `nodeNoteSidebar` | 改造 |
| `src/styles/mindmap.css` | 自有层：删被生成层取代的规则 | 改造 |
| `src/windows/MindMap/MindMapApp.vue` | 挂载点调整（三岛 / 单壳 / 模态 / 底栏） | 改造 |

---

## Task 1: 导图名规范化纯函数 + 文件名岛组件

**Files:**
- Create: `src/utils/mindmapName.ts`
- Create: `src/utils/mindmapName.test.ts`
- Create: `src/windows/MindMap/chrome/MmFilenameIsland.vue`

**Interfaces:**
- Produces：`normalizeMapName(raw: string): string`——去首尾空白、折叠内部连续空白、裁剪到 32 字符、空串回退 `'未命名导图'`
- Produces：`MmFilenameIsland` 组件，props `{ name: string }`，emits `rename(newName: string)`

- [ ] **Step 1: 写失败的测试**

`src/utils/mindmapName.test.ts`：

```ts
import test from 'node:test'
import assert from 'node:assert/strict'
import { normalizeMapName } from './mindmapName'

test('normalizeMapName：去首尾空白、折叠内部空白', () => {
  assert.equal(normalizeMapName('  项目   规划  '), '项目 规划')
})

test('normalizeMapName：裁剪到 32 字符', () => {
  const long = '一'.repeat(40)
  assert.equal(normalizeMapName(long).length, 32)
  // 32 字符以内原样返回
  assert.equal(normalizeMapName('短名字'), '短名字')
})

test('normalizeMapName：空串与纯空白回退为「未命名导图」', () => {
  assert.equal(normalizeMapName(''), '未命名导图')
  assert.equal(normalizeMapName('   \n\t '), '未命名导图')
})
```

- [ ] **Step 2: 运行测试确认失败**

Run: `pnpm test:unit 2>&1 | grep -E "mindmapName|Cannot find"`

Expected: FAIL —— `Cannot find module './mindmapName'`（连带 `TS7006` 级联报错属正常）

- [ ] **Step 3: 写最小实现**

`src/utils/mindmapName.ts`：

```ts
/**
 * 导图名规范化（原型文件名岛 `#mmFilenameInput` 的 `maxlength="32"` 口径）。
 *
 * 原型只在 HTML 上限了长度（`docs/index.html:335`），没有 JS 侧处理；空名回退
 * 「未命名导图」是原型 `openMindmapEditor` 里对根节点文本为空时的兜底
 * （`docs/app.js:1697-1698`）。放在这里是为了让「提交什么名字」这件事可单测。
 */

/** 名称长度上限（与原型 `maxlength="32"` 一致）。 */
export const MAP_NAME_LIMIT = 32

/** 空名兜底文案（原型同款）。 */
export const MAP_NAME_FALLBACK = '未命名导图'

/**
 * 规范化导图名：去首尾空白 → 内部连续空白折叠为单个空格 → 截断到 32 字符 → 空则回退。
 * 截断按 UTF-16 码元计（与浏览器 `maxlength` 的行为一致，中文与 emoji 都按原型来）。
 */
export function normalizeMapName(raw: string): string {
  const collapsed = raw.trim().replace(/\s+/g, ' ')
  const clipped = collapsed.slice(0, MAP_NAME_LIMIT)
  return clipped || MAP_NAME_FALLBACK
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `pnpm test:unit 2>&1 | grep -E "ℹ (tests|pass|fail)"`

Expected: `tests` +3、`fail 0`

- [ ] **Step 5: 写文件名岛组件**

`src/windows/MindMap/chrome/MmFilenameIsland.vue`：

```vue
<script setup lang="ts">
import { nextTick, ref } from 'vue'
import { logger } from '@/service/logger'
import { MAP_NAME_LIMIT, normalizeMapName } from '@/utils/mindmapName'

/**
 * 文件名岛（原型 `#mmFilenameIsland`，`docs/index.html:333-346`）。
 *
 * 形态照原型：常态是文本 + ✏️ 按钮，编辑态是输入框 + 💾 按钮；Enter / 失焦 / 💾 提交，
 * Esc 取消。**不渲染后缀徽章与后缀下拉**（spec D44）：我们的导图存进笔记
 * （`notes.content` 即导图名、`mindmap_data` 是导图数据），没有「保存格式」概念，
 * 原型那 4 个后缀实测也只是字符串、落库无差异。
 */
const props = defineProps<{ name: string }>()
const emit = defineEmits<{ rename: [name: string] }>()

const editing = ref(false)
const draft = ref('')
const input = ref<HTMLInputElement | null>(null)

async function startEdit(): Promise<void> {
  draft.value = props.name
  editing.value = true
  await nextTick()
  input.value?.focus()
  input.value?.select()
  logger.debug('mindmap', '文件名岛进入编辑态')
}

function commit(): void {
  if (!editing.value) return
  const name = normalizeMapName(draft.value)
  editing.value = false
  if (name === props.name) return
  logger.info('mindmap', `导图改名为「${name}」`)
  emit('rename', name)
}

function cancel(): void {
  editing.value = false
  logger.debug('mindmap', '文件名岛放弃改名')
}
</script>

<template>
  <div class="mm-filename-island" @keydown.esc.stop="cancel">
    <template v-if="editing">
      <input
        ref="input"
        v-model="draft"
        class="mm-filename-input"
        :maxlength="MAP_NAME_LIMIT"
        spellcheck="false"
        autocomplete="off"
        @keydown.enter.prevent="commit"
        @blur="commit"
      />
      <button type="button" class="mm-filename-btn" title="保存名称" @mousedown.prevent @click="commit">💾</button>
    </template>
    <template v-else>
      <span class="mm-filename-text" title="点击编辑导图名" @click="startEdit">{{ name }}</span>
      <button type="button" class="mm-filename-btn" title="编辑导图名" @click="startEdit">✏️</button>
    </template>
  </div>
</template>
```

> 说明：`@mousedown.prevent` 是防止按钮抢焦点触发输入框 `blur`（blur 也会 commit，会重复提交）。

- [ ] **Step 6: 校验并提交**

```bash
pnpm typecheck && pnpm format:check && pnpm test:unit 2>&1 | grep -E "ℹ (tests|pass|fail)"
git add src/utils/mindmapName.ts src/utils/mindmapName.test.ts src/windows/MindMap/chrome/MmFilenameIsland.vue
git commit -m "feat(mindmap): 导图名规范化纯函数与文件名岛组件"
```

---

## Task 2: 顶栏三岛重构（左岛 / 文件名岛 / 右岛）

**Files:**
- Modify: `src/windows/MindMap/chrome/Toolbar.vue`（211 行 → 三岛）
- Modify: `src/windows/MindMap/MindMapApp.vue:330-339`（标题栏按钮迁走）、`:353`（Toolbar 挂载）、`:176-205`（`save()` 复用）、新增 `renameTo()`
- Modify: `src/windows/MindMap/core/store.ts`（若需要新的 bus 事件）
- Modify: `src-tauri/src/app/windows.rs`（`MINDMAP_SIZE` 960×700 → **1360×900**；三段式顶栏实测需 1325–1525px，见 spec D51）
- Modify: `src/styles/mindmap.css`（顶栏降级布局：左岛可压缩滚动、中/右岛不压缩，见 spec D51）

**Interfaces:**
- Consumes：Task 1 的 `MmFilenameIsland`（props `name`，emits `rename`）
- Produces：`Toolbar.vue` 根元素为 `.mm-topbar`（内含 `.mm-island.mm-island-left` / `MmFilenameIsland` / `.mm-island.mm-island-right`）；新增 props `mapName: string`、emits `rename` / `save` / `close` / `new-map`（**不走 bus**——模板里就是组件 emit，2026-09-14 订正：原 Interfaces 行误记为新 bus 事件）

- [ ] **Step 1: 改 `Toolbar.vue` 的模板为三岛**

把现有单一 `.mm-toolbar` 根元素替换为：

```vue
<template>
  <div class="mm-topbar">
    <!-- 左岛：节点与内容编辑工具组（原型 docs/index.html:313-330，去掉 AI） -->
    <div class="mm-island mm-island-left">
      <button
        v-for="btn in leftButtons"
        :key="btn.key"
        type="button"
        class="mm-tool-btn"
        :class="{ danger: btn.danger }"
        :title="btn.label"
        :disabled="btn.disabled?.()"
        @click="btn.run"
      >
        <i class="iconfont" :class="btn.icon" />
        <span class="mm-txt">{{ btn.label }}</span>
      </button>
    </div>

    <!-- 中间：文件名岛（原型 docs/index.html:333-346） -->
    <MmFilenameIsland :name="mapName" @rename="emit('rename', $event)" />

    <!-- 右岛：文件与存储（原型 docs/index.html:349-359） -->
    <div class="mm-island mm-island-right">
      <button type="button" class="mm-tool-btn" title="保存到 Inkling 笔记（Ctrl+S）" @click="emit('save')">
        <i class="iconfont iconlingcunwei" /><span class="mm-txt">保存</span>
      </button>
      <button type="button" class="mm-tool-btn" title="新建导图（清空当前内容）" @click="emit('new-map')">
        <i class="iconfont iconxinjian" /><span class="mm-txt">新建</span>
      </button>
      <button type="button" class="mm-tool-btn" title="导入 JSON 导图" @click="bus.emit('showImport')">
        <i class="iconfont icondaoru" /><span class="mm-txt">导入</span>
      </button>
      <button type="button" class="mm-tool-btn" title="导出思维导图（选择格式、存储路径与文件名）" @click="bus.emit('showExport')">
        <i class="iconfont icondaochu" /><span class="mm-txt">导出</span>
      </button>
      <button type="button" class="mm-tool-btn" title="关闭窗口" @click="emit('close')">
        <i class="iconfont iconguanbi" /><span class="mm-txt">关闭</span>
      </button>
    </div>
  </div>
</template>
```

> **图标类名必须真实存在**（图标字体在 `src/assets/mindmap/icon-font/iconfont.css`，由 `mindmap.ts:11` 引入）。上面五个已逐个核实：`iconlingcunwei`（存／最常见的「保存」字形）、`iconxinjian`、`icondaoru`、`icondaochu`、`iconguanbi`。左岛沿用 `Toolbar.vue` 里**已有**的类名（`iconhoutui-shi` / `iconqianjin` / `icongeshihua` / `iconjiedian` / `icontianjiazijiedian` / `iconshanchu` / `iconimage` / `icon` / `iconlianjie` / `iconfujian` / `iconbiaoqian` / `iconchaolianjie` / `icongongshi` / `iconwaikuang` 等），**不要凭印象造新类名**。
>
> 形态说明：原型这些按钮用的是 emoji + `.mm-ico`（`<span class="mm-ico">💾</span>`）。我们**沿用现有 iconfont 矢量图标**——形态（三岛结构）照原型，字形技术沿用现状：换成 emoji 会让整个窗口的图标语言与现状不一致，且属于纯外观变动，不在本阶段范围（可后续单独提）。

> **⚠️ 布局必读**：生成层里 `.mm-filename-island` 仍是原型「单岛居中」时代的定位——`src/styles/components.css:688-692` 的 `position:absolute; left:50%; top:10px; transform:translateX(-50%)`。改成「左岛 + 中岛 + 右岛」的横向排布时，**必须在自有层 `src/styles/mindmap.css` 覆盖它**（例如给 `.mm-topbar` 加 `display:flex; align-items:center; gap:10px`，并让 `.mm-filename-island` 回到静态流 + `margin:0 auto`）。
> **不要改 `docs/styles.css` 再 `sync:styles`，也不要手改 `components.css`**——`docs/` 是设计师的原型参照（只读），生成层由脚本生成（手改会被 `sync:styles --check` 判死）。项目适配一律进自有层，这是前四个阶段的惯例（`window-fit.css` 就是干这个的）。

`<script setup>` 顶部补齐：

```ts
import MmFilenameIsland from './MmFilenameIsland.vue'

const props = defineProps<{ mapName: string }>()
const emit = defineEmits<{ rename: [name: string]; save: []; close: []; 'new-map': [] }>()
```

`leftButtons` 由现有 `nodeButtons` + `fileButtons` 重排而成，**顺序按原型**（回退/前进/格式刷/同级节点/子节点/删除节点/图片/图标/超链接/**链接节点**/**附件**/备注/标签/概要/关联线/公式/外框），其中：

- **删除节点** 补 `danger: true`（让生成层 `.mm-tool-btn.danger:hover`（`components.css:788`）生效——此前一直空转）；
- **链接节点 / 附件** 保留（我们的真能力，原型没有，spec 差异表 #2）；
- **不放 AI**（spec §1）；
- `fileButtons` 里只保留「回退 / 前进 / 格式刷」三项（其余已迁到右岛）。

- [ ] **Step 2: `MindMapApp.vue` 接线**

```vue
<Toolbar
  v-if="!ui.isZenMode"
  :map-name="mapName"
  @rename="renameTo"
  @save="save()"
  @close="close"
  @new-map="startNewMap"
/>
```

标题栏（`:330-339`）删掉「保存」「关闭」两个按钮（已迁到右岛），保留 🧠 标题 + `TagList`；标题文本改用 `mapName`。

`<script setup>` 新增：

```ts
/** 未保存的新导图上改的名字（此时还没有笔记，先记在本地，保存时写入 `content`）。 */
const pendingName = ref('')

/** 文件名岛显示的名字：已有笔记用笔记标题；尚未保存的新导图用本地暂存名。 */
const mapName = computed(() => note.value?.content || pendingName.value || MAP_NAME_FALLBACK)

/**
 * 文件名岛改名。
 *
 * 已有笔记：立即写回 `content`（`save()` 写的就是这个字段），主窗口经 notes-changed 刷新列表。
 * 尚未保存的新导图：**只记在本地**——不能为了改名就静默建一条笔记，那会破坏
 * `markDirtyAndAutosave` 里「新建导图首次必须手动保存」的约定（`:168` 注释）。
 */
async function renameTo(name: string): Promise<void> {
  if (!noteId.value) {
    pendingName.value = name
    logger.info('mindmap', `新导图暂存名「${name}」，保存时写入`)
    return
  }
  try {
    await api.notes.save({
      id: noteId.value,
      content: name,
      tags: [...tags.value],
      editorMode: 'mindmap',
      mindmapData: mindmapData.value ?? serializeMindMapData(initialData.value),
      draft: false,
    })
    toast('已重命名')
    logger.info('mindmap', `导图改名为「${name}」`)
  } catch (error) {
    logger.error('mindmap', '导图改名失败', error)
    toast('重命名失败')
  }
}

/** 新建：原型 `#mmNew`（`docs/app.js:1748-1756`）——确认后清空当前内容为空白导图。 */
function startNewMap(): void {
  if (!window.confirm('新建导图将清空当前未保存的内容，确认新建？')) return
  const instance = mindMap.value
  if (!instance) return
  noteId.value = ''
  pendingName.value = ''
  mindmapData.value = null
  instance.setData({ data: { text: '中心主题' } })
  instance.view.fit()
  dirty.value = false
  logger.info('mindmap', '已清空为新建导图')
}
```

**并且 `save()` 里把 `content` 换成 `mapName.value`**（原为 `note.value?.content ?? ''`），这样新导图保存时会把暂存名一并写入；保存成功后 `pendingName.value = ''` 清空暂存。

> `setData` 的入参形状以现有 `core/persistence.ts` 的 `parseMindMapData` 默认值为准（实现时读该文件对齐，勿凭记忆写）。

- [ ] **Step 3: 校验**

```bash
pnpm typecheck && pnpm format:check && pnpm sync:styles --check
```

Expected: 全绿（本步未碰 CSS 文件，只用了生成层已有的 `.mm-topbar` / `.mm-island` / `.mm-tool-btn` / `.mm-filename-*` 类名）

- [ ] **Step 4: 实机看一眼（不写验收，只看是否散架）**

Run: `pnpm tauri:dev` → 主窗口「笔记」页点「🧠 思维导图」开窗

Expected: 顶栏出现左岛 + 文件名岛 + 右岛三段；点 ✏️ 能改名且名字回显；点「新建」弹确认。**若布局明显错乱（生成层规则与 DOM 层级不匹配），在本步就地修 `mindmap.css`（自有层），并在注释里注明「待 Task 7 收敛」。**

- [ ] **Step 5: 提交**

```bash
git add src/windows/MindMap/chrome/Toolbar.vue src/windows/MindMap/MindMapApp.vue
git commit -m "feat(mindmap): 顶栏按原型改为双工具岛 + 新建文件名岛（含重命名与新建）"
```

---

## Task 3: 单抽屉壳（10 个实例 → 1 个）

> **⚠️ 实施期订正（2026-09-14，实现者开工前发现）**：本节原先假设 `MindMapApp.vue` 直接使用 `SidebarShell`——**实际是每个侧栏组件自己包自己**（`ShortcutSidebar` / `StructureSidebar` / `ThemeSidebar` / `BaseStyleSidebar` / `NodeStyleSidebar` / `SettingSidebar` / `IconSidebar` / `FormulaSidebar` / `OutlineSidebar` / `NoteSidebar` 各写 `<SidebarShell name title>内容</SidebarShell>`，`MindMapApp` 只挂 `<XxxSidebar />` 不传 props）。照原方案做会得到**抽屉套抽屉**。已批准的做法（方案 A）：
> 1. `git mv SidebarShell.vue → MmDrawer.vue` 并改造为分发壳；
> 2. **9 个侧栏组件去掉自带的 `<SidebarShell>` 外壳与 import**，只留 body 内容（**本节的额外改动范围**）；
> 3. `MindMapApp.vue` 换一行 `<MmDrawer v-if="!ui.isZenMode" />`；
> 4. **四处**必要补充（原写「三处」，Task 3 审查期间补上第 4 条）：图标/公式/快捷键三项的标题映射（`sidebarTriggerList` 只有 dock 六项）、自有层给 `.mm-drawer-body` 补 `display:flex; flex-direction:column; gap:10px`（原侧栏表单件靠父级 gap 撑开）、过渡名改 `mm-drawer-slide`、**自有层给 `.mm-drawer` 补 `z-index: 35`**（生成层是 10，而 Task 2 把顶栏抬到了 30 ⇒ 抽屉 header 落到顶栏纵向带内，右岛不透明且 `pointer-events:auto` 会把 `✕` 盖死；35 落在 30 与 40 之间的唯一空档）；
> 5. 动态渲染包 **`<KeepAlive>`**：现状是 10 个侧栏常驻挂载、切换不丢状态；不包会退化成「切侧栏卸载重挂」（大纲树展开态重置等），那是**对现状的回归**。KeepAlive 缓存的是组件实例、DOM 里仍只有一个抽屉，收益与状态兼得。

**Files:**
- Rename+Modify: `src/windows/MindMap/sidebars/SidebarShell.vue` → `MmDrawer.vue`（改造为内容分发壳）
- Modify: `src/windows/MindMap/MindMapApp.vue:369-378`（10 个 `<XxxSidebar />` → 1 个 `<MmDrawer />`）
- Modify: **9 个侧栏组件**（去掉自带的 `<SidebarShell>` 外壳与 import，只留 body 内容）
- Modify: `src/styles/mindmap.css`（`.mm-drawer-body` 的 flex+gap）
- Delete: `src/windows/MindMap/sidebars/NoteSidebar.vue`
- Modify: `src/windows/MindMap/core/store.ts:5-16`（`SidebarName` 去掉 `nodeNoteSidebar`）

**Interfaces:**
- Consumes：`ui.activeSidebar`（单值）、9 个侧栏组件（dock 六项 + 图标 / 公式 / 快捷键三个过渡项）
- Produces：`MmDrawer.vue`（原名 `SidebarShell.vue`），按 `ui.activeSidebar` 用 `<KeepAlive><component :is>` 渲染对应侧栏，根元素 `.mm-drawer`

- [ ] **Step 1: 改造 `SidebarShell.vue` 为单壳**

```vue
<script setup lang="ts">
import { computed } from 'vue'
import BaseStyleSidebar from './BaseStyleSidebar.vue'
import NodeStyleSidebar from './NodeStyleSidebar.vue'
import OutlineSidebar from './OutlineSidebar.vue'
import SettingSidebar from './SettingSidebar.vue'
import StructureSidebar from './StructureSidebar.vue'
import ThemeSidebar from './ThemeSidebar.vue'
import { useMindMap } from '../core/useMindMap'
import { sidebarTriggerList } from '../constants/lists'

/**
 * 右侧抽屉壳（原型单个 `#mmDrawer`，`docs/index.html:383-389`）。
 *
 * 原型是「一个抽屉换 title/body 内容」，改造前我们是 10 个壳实例同存 DOM、靠
 * `ui.activeSidebar` 单值做视觉互斥——同存 10 套表单（含 480 行的节点样式面板）
 * 纯属浪费。这里收敛为单壳：壳常量，内容按 `activeSidebar` 动态渲染。
 *
 * 只渲染 dock 六项（`sidebarTriggerList`）对应的侧栏；图标 / 公式已改为模态、
 * 快捷键表走底栏「更多」下拉，都不再走本壳。
 */
const { ui } = useMindMap()

const CONTENT = {
  nodeStyle: NodeStyleSidebar,
  baseStyle: BaseStyleSidebar,
  theme: ThemeSidebar,
  structure: StructureSidebar,
  outline: OutlineSidebar,
  setting: SettingSidebar,
  // 以下三项是过渡：图标/公式在 Task 4 迁到模态、快捷键在 Task 5 保持不变但入口在底栏，
  // 收敛壳的这一步先把它们一并纳进来，**保证本任务结束后每一项入口都仍然可用**
  // （否则 Task 3 结束到 Task 4/5 之间，点「图标」「公式」「更多」会没有任何反应）。
  nodeIconSidebar: IconSidebar,
  formulaSidebar: FormulaSidebar,
  shortcutKey: ShortcutSidebar,
} as const

const current = computed(() => CONTENT[ui.activeSidebar as keyof typeof CONTENT] ?? null)
const title = computed(() => sidebarTriggerList.find((item) => item.value === ui.activeSidebar)?.name ?? '')
</script>

<template>
  <Transition name="mm-drawer-slide">
    <aside v-if="current" class="mm-drawer">
      <div class="mm-drawer-header">
        <span class="mm-drawer-title">{{ title }}</span>
        <button type="button" class="mm-drawer-close" title="关闭面板" @click="ui.activeSidebar = ''">✕</button>
      </div>
      <div class="mm-drawer-body">
        <component :is="current" />
      </div>
    </aside>
  </Transition>
</template>
```

> 过渡名从 `mm-sidebar-slide` 改为 `mm-drawer-slide`：在 `mindmap.css` 里把同一条动画规则改名（自有层），Task 7 再决定是否删。

- [ ] **Step 2: `MindMapApp.vue` 换挂载**

删掉 `:369-378` 的 10 行 `<XxxSidebar />`（含 `NoteSidebar`），替换为一行 `<MmDrawer v-if="!ui.isZenMode" />`；同步删除这些侧栏组件的 import——**但 `IconSidebar` / `FormulaSidebar` / `ShortcutSidebar` 三个的 import 要在本步补进 `SidebarShell.vue`**（见上一步的 CONTENT 过渡项），它们在 Task 4/5 才迁走。

- [ ] **Step 3: 删死码**

```bash
git rm src/windows/MindMap/sidebars/NoteSidebar.vue
```

`core/store.ts` 的 `SidebarName` 联合类型删掉 `| 'nodeNoteSidebar'`；全仓 grep 确认无残留：

```bash
grep -rn "nodeNoteSidebar\|NoteSidebar" src/ || echo "无残留引用"
```

Expected: `无残留引用`

- [ ] **Step 4: 校验并提交**

```bash
pnpm typecheck && pnpm format:check
git add -A src/windows/MindMap
git commit -m "refactor(mindmap): 侧栏壳收敛为单抽屉，删除无入口的 NoteSidebar"
```

---

## Task 4: 图标与公式改为模态

**Files:**
- Create: `src/windows/MindMap/popups/MmModal.vue`
- Create: `src/windows/MindMap/popups/NodeIconModal.vue`
- Create: `src/windows/MindMap/popups/NodeFormulaModal.vue`
- Modify: `src/windows/MindMap/chrome/Toolbar.vue`（图标 / 公式按钮改发模态事件）
- Modify: `src/windows/MindMap/MindMapApp.vue`（挂载两个模态）
- Delete: `src/windows/MindMap/sidebars/IconSidebar.vue`、`FormulaSidebar.vue`（内容迁走后）

**Interfaces:**
- Consumes：`bus.emit('showNodeIcon', node)` / `bus.emit('showNodeFormula', node)`（现有事件名不变）
- Produces：`MmModal` props `{ title: string; visible: boolean }`，emits `close`；插槽 `default`（body）与 `footer`

- [ ] **Step 1: 写通用模态壳**

`src/windows/MindMap/popups/MmModal.vue`：

```vue
<script setup lang="ts">
/**
 * 通用节点弹窗壳（原型 `#mmModalOverlay` + `.mm-modal`，`docs/index.html:398-410`）。
 *
 * 原型用一个遮罩复用 8 种表单；我们此前每个表单各自成组件（差异大，保留），
 * 只有图标与公式是从右侧抽屉改过来的（spec D42），因此这里只抽这两个共用的壳。
 */
defineProps<{ title: string; visible: boolean }>()
const emit = defineEmits<{ close: [] }>()
</script>

<template>
  <div v-if="visible" class="mm-modal-overlay" @click.self="emit('close')">
    <div class="mm-modal">
      <div class="mm-modal-header">
        <span>{{ title }}</span>
        <button type="button" class="mm-modal-close" title="关闭" @click="emit('close')">✕</button>
      </div>
      <div class="mm-modal-body"><slot /></div>
      <div class="mm-modal-footer"><slot name="footer" /></div>
    </div>
  </div>
</template>
```

- [ ] **Step 2: 迁移图标内容**

把 `sidebars/IconSidebar.vue` 的表单体（图标分组与点击应用逻辑，`:68` 起）迁入 `popups/NodeIconModal.vue`，模板换成：

```vue
<template>
  <MmModal :visible="visible" title="节点图标" @close="emit('close')">
    <div v-for="group in groups" :key="group.name" class="mm-icon-grid">
      <button
        v-for="icon in group.icons"
        :key="icon"
        type="button"
        class="mm-icon-tile"
        :class="{ active: current === icon }"
        @click="apply(icon)"
      >{{ icon }}</button>
    </div>
  </MmModal>
</template>
```

> **保留我们的库原生图标集**（`simple-mind-map` 的 icon 插件），不照抄原型写死的 emoji 列表；点击即应用并关窗（这一点与原型一致，`app.js:1927-1929`）。

- [ ] **Step 3: 迁移公式内容**

同理迁 `FormulaSidebar.vue` 到 `popups/NodeFormulaModal.vue`，body 用生成层 `.mm-formula-table`，保留我们的 LaTeX 输入与常用公式表。

- [ ] **Step 4: 接线 + 删旧文件**

- `Toolbar.vue`：图标/公式按钮由 `openSidebar('nodeIconSidebar' | 'formulaSidebar')` 改为 `bus.emit('showNodeIcon' | 'showNodeFormula')`；
- `MindMapApp.vue`：挂 `<NodeIconModal />` 与 `<NodeFormulaModal />`（与其它弹层同批，`:359-368`）；
- `git rm src/windows/MindMap/sidebars/IconSidebar.vue src/windows/MindMap/sidebars/FormulaSidebar.vue`；
- `core/store.ts` 的 `SidebarName` 删掉 `nodeIconSidebar` / `formulaSidebar` 两个取值。

- [ ] **Step 5: 校验并提交**

```bash
grep -rn "nodeIconSidebar\|formulaSidebar\|IconSidebar\|FormulaSidebar" src/ || echo "无残留引用"
pnpm typecheck && pnpm format:check
git add -A src/windows/MindMap
git commit -m "feat(mindmap): 图标与公式改为工具栏模态，抽通用 MmModal 壳"
```

---

## Task 5: 底栏重排 + 右键菜单挂 body

**Files:**
- Create: `src/windows/MindMap/chrome/MmBottombar.vue`
- Modify: `src/windows/MindMap/MindMapApp.vue:354,358`（`NavigatorToolbar` + `Count` → `MmBottombar`）
- Modify: `src/windows/MindMap/popups/ContextMenu.vue`（`Teleport to="body"`）

**Interfaces:**
- Consumes：`Count.vue`、`NavigatorToolbar.vue` 现有内容（只搬位置，不改行为）
- Produces：`MmBottombar.vue`，根元素 `.mm-bottombar`（左 `.mm-stat-left`，右 `.mm-ctrl-right`）

- [ ] **Step 1: 合并为底栏**

```vue
<template>
  <div class="mm-bottombar">
    <div class="mm-stat-left"><Count /></div>
    <div class="mm-ctrl-right">
      <!-- 按原型顺序（docs/index.html:416-432）：定位中心 / 搜索 / 只读 / 小地图 / 全屏
           ｜ 缩放 −/百分比/+ / 自适应 / 展开全部 / 收起全部
           不放语言下拉（原型死控件）；保留「鼠标行为」「演示」两项（我们的真能力） -->
    </div>
  </div>
</template>
```

按钮用生成层的 `.mm-ctrl-btn` / `.mm-ctrl-sep` / `.mm-zoom-val`；各按钮的 `@click` 全部沿用 `NavigatorToolbar.vue` 现有实现（`MouseAction` / `Fullscreen` / `Scale` / `Demonstrate` 四个子组件原样搬进来）。

- [ ] **Step 2: 右键菜单改挂 body**

`popups/ContextMenu.vue` 的模板包一层：

```vue
<template>
  <Teleport to="body">
    <!-- … 现有菜单内容，class 由 .mm-ctxmenu 改为生成层的 .mm-ctx / .mm-ctx-item … -->
  </Teleport>
</template>
```

定位改用 `clientX/clientY`（body 级 fixed）；并在 `mindmap.css` 里把 `.mm-ctx` 用到的通用令牌（`--wsa` / `--text-strong` / `--text-dim` / `--red-soft`）**覆写为 `--mm-*` 系列**——原型此处用通用令牌，会导致「菜单跟随 31 套主题、窗口外壳跟随 `--mm-*`」不同步（spec 差异表 #14）。

- [ ] **Step 3: 校验并提交**

```bash
pnpm typecheck && pnpm format:check
git add -A src/windows/MindMap
git commit -m "feat(mindmap): 底栏按原型重排为单条控制栏，右键菜单改挂 body"
```

---

## Task 6: 30 套主题接入 `--mm-*` + 修 naive 浅色基底

> **⚠️ 本任务因 Task 2/3 而变成阻塞项（2026-09-14）**：Task 2（顶栏三岛）与 Task 3（单抽屉壳）已把窗口外壳迁到生成层类名，而生成层的 `.mm-topbar` / `.mm-island` / `.mm-drawer` / `.mm-search-box` / `.mm-modal*` **全部走 `--mm-*`**，`--mm-*` 又只在 `:root`（原型抄来的**浅色**默认）与 `typewriter` 有定义 ⇒ **在 13 套 `--scheme: dark` 主题下，这些外壳是白底而内容是浅字**。Task 3 审查已把「13 套深色主题下抽屉正文不可读」报为 Important 并认领给本任务；Task 2 审查同样报过「dark 主题下岛屿白底白字」。
> 因此本任务**不是可选完善**：它决定窗口在深色主题下能不能用。Task 4/5 之后、Task 8 实机验收之前必须落地；Task 4/5 期间**不要在深色主题下做观感判断**。

**Files:**
- Modify: `src/styles/extensions.css`（**自有层**：新增一节，给 30 套主题各补 13 个 `--mm-*`；`sepia` 也在同一节里）
- Modify: `src/windows/MindMap/core/naiveTheme.ts`
- Create: `src/utils/themeTokens.test.ts`（断言每套主题都定义了 13 个 `--mm-*`）

> **为什么不改 `docs/styles.css`**：`docs/` 是设计师给的原型参照，前四个阶段的惯例是**只读它、不写它**（项目适配一律进自有层；`extensions.css:710-712` 就留着一条「已建议在 docs/styles.css 改为 …」而未自行改动）。`themes.css` 由 `docs/styles.css` 生成、不得手改，因此这 30 套覆盖要写在 `extensions.css`——它在加载链里位于 `themes.css` **之后**（`index.ts` 顺序：tokens → base → components → themes → **extensions** → window-fit → motion → glass），能压过生成层，且已有 §7 棕褐主题、§9–§11 各阶段补丁的同款先例。

**Interfaces:**
- Consumes：主题块既有令牌 `--wsa` / `--menu-bg` / `--glass-bg` / `--accent` / `--accent-rgb` / `--text` / `--text-strong` / `--text-dim` / `--red` / `--scheme`
- Produces：每套主题 13 个 `--mm-*`；`naiveTheme.ts` 的 `buildThemeOverrides()` 返回按 `--scheme` 选基底的 overrides

- [ ] **Step 1: 写失败的测试**

`src/utils/themeTokens.test.ts`（读 `docs/styles.css` 文本做静态断言，不需要浏览器）：

```ts
import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

const MM_TOKENS = [
  '--mm-surface', '--mm-surface-2', '--mm-surface-3', '--mm-border',
  '--mm-text', '--mm-text-2', '--mm-text-dim',
  '--mm-primary', '--mm-primary-soft', '--mm-danger', '--mm-danger-soft',
  '--mm-accent-2', '--mm-accent-2-rgb',
] as const

test('每套主题都有全部 13 个 --mm-* 令牌', () => {
  // 覆盖来自两处：docs/styles.css 的 typewriter 块（经生成层进 themes.css）与
  // src/styles/extensions.css 的其余主题（含 dark 与 sepia）。
  // 注意 dark：它**必须**有显式的 [data-theme='dark'] 块——不能靠无 data-theme 的 :root 兜底，
  // 因为那个 :root 的 --mm-* 是原型抄来的浅色值（深色主题下窗口会变浅，Task 2 实机发现）。
  const extensionCss = readFileSync('src/styles/extensions.css', 'utf8')
  const docCss = readFileSync('docs/styles.css', 'utf8')
  for (const key of themeKeys()) {
    const block = blockOf(key)
    assert.ok(block, `找不到主题 ${key} 的 --mm-* 定义块`)
    const missing = MM_TOKENS.filter((token) => !block.includes(`${token}:`))
    assert.deepEqual(missing, [], `主题 ${key} 缺少：${missing.join(', ')}`)
  }

  /** 主题清单以 constants/themes.ts 为唯一真源。 */
  function themeKeys(): string[] {
    return [...readFileSync('src/constants/themes.ts', 'utf8').matchAll(/key: '([^']+)'/g)].map((m) => m[1])
  }

  /** 取某主题的令牌定义块：typewriter 在 docs/styles.css，其余在 extensions.css。 */
  function blockOf(key: string): string | undefined {
    const source = key === 'typewriter' ? docCss : extensionCss
    const matched = new RegExp(`:root\\[data-theme='${key}'\\]\\s*\\{([\\s\\S]*?)\\n\\}`).exec(source)
    return matched?.[1]
  }
})
```

- [ ] **Step 2: 运行确认失败**

Run: `pnpm test:unit 2>&1 | grep -E "mm-|fail"`

Expected: FAIL —— 30 个主题块里除 `typewriter` 外全部报缺少 13 个令牌

- [ ] **Step 3: 给 30 套主题 + `dark` 补令牌**

⚠️ **`dark` 也必须显式覆盖**（Task 2 的实机发现）：`dark` 主题没有自己的块、落到 `tokens.css` 的 `:root`，而那个 `:root` 里的 `--mm-*` 是从原型逐字抄来的**浅色**值（`#ffffff` / `#333` / Ant 蓝）。Task 2 把顶栏换成原型类名后顶栏就开始吃 `--mm-*`——**若不覆盖 `dark`，深色主题下导图窗口会变成全站唯一的浅色窗口**。因此要在自有层写一个 `:root[data-theme='dark']` 块。

在 `src/styles/extensions.css` 末尾**新增一节**（照 §7/§9/§10/§11 的分节风格），对 `src/constants/themes.ts` 里**除 `typewriter` 外**的每套主题各写一个块（`dark` 与 `sepia` 都在本节内）。取值全部由该主题**既有**令牌推导，不手挑颜色：

```css
/* ═══ 12. 思维导图窗口令牌（阶段五 D46） ═══
 * 原型导图窗口的 chrome（外壳 / 工具岛 / 抽屉 / 模态 / 表单 / 底栏）全部走 --mm-*，
 * 而原型自身只在 :root 与 [data-theme="typewriter"] 定义了它们（docs/styles.css:101-113 / 2564-2576，
 * 那两处已由生成层带入 tokens.css 与 themes.css）——其余 30 套主题没有覆盖，
 * 导图窗口因此是全站唯一不跟随主题换肤的窗口（原型 styles.css:95-100 注释自陈）。
 * 本节即兑现它的「⚠️ 下一步」。写在自有层而非 docs/：docs/ 是设计师的原型参照，只读不写。 */
:root[data-theme='<每套主题的 key>'] {
  --mm-surface: var(--menu-bg);
  --mm-surface-2: var(--glass-bg);
  --mm-surface-3: rgba(var(--wsa), 0.08);
  --mm-border: rgba(var(--wsa), 0.16);
  --mm-text: var(--text);
  --mm-text-2: var(--text-strong);
  --mm-text-dim: var(--text-dim);
  --mm-primary: var(--accent);
  --mm-primary-soft: rgba(var(--accent-rgb), 0.14);
  --mm-danger: var(--red);
  --mm-danger-soft: color-mix(in srgb, var(--red) 16%, transparent);
  --mm-accent-2: var(--accent);
  --mm-accent-2-rgb: var(--accent-rgb);
}
```

> `color-mix` 项目已在用（`extensions.css:746`），WebView2 无兼容顾虑。
> 主题 key 清单取自 `src/constants/themes.ts`（32 套里除 `dark`——它落到 `tokens.css` 的 `:root`，无需覆盖——与已覆盖的 `typewriter`）。

- [ ] **Step 4: 跑测试确认通过**

```bash
pnpm test:unit 2>&1 | grep -E "ℹ (tests|pass|fail)"
pnpm sync:styles --check
```

Expected: 测试全绿；`sync:styles --check` 通过（本步**没有**改 `docs/`，生成层原样不动）

- [ ] **Step 5: 修 naive 的两处硬编码**

`src/windows/MindMap/core/naiveTheme.ts`：

```ts
/**
 * 按当前主题的明暗选基底（原实现恒用 darkTheme，浅色主题下 naive 默认底色仍是暗的）。
 * 依据是主题块里的 `--scheme`（30 个主题块都定义了它，`themes.css`）。
 */
function currentScheme(): 'light' | 'dark' {
  return cssVar('--scheme', '').trim() === 'light' ? 'light' : 'dark'
}

// buildThemeOverrides 内：
const overrides = {
  common: {
    // …既有的 primaryColor / textColor* / popoverColor 等保持不变…
    inputColor: 'rgba(var(--wsa), 0.06)',
    borderColor: 'rgba(var(--wsa), 0.14)',
  },
}
return { theme: currentScheme() === 'light' ? null : naiveDark, overrides }
```

> `naiveDark` 保持导出（`MindMapApp.vue:327` 的 `:theme` 绑定改为读 `buildThemeOverrides()` 一并返回的 `theme`），避免两处各判一次明暗。

- [ ] **Step 6: 校验并提交**

```bash
pnpm typecheck && pnpm format:check && pnpm test
git add src/styles/extensions.css src/windows/MindMap/core/naiveTheme.ts src/windows/MindMap/MindMapApp.vue src/utils/themeTokens.test.ts
git commit -m "feat(mindmap): 其余 30 套主题接入 --mm-* 令牌，naive 基底按主题明暗选择"
```

---

## Task 7: 自有层样式收敛 + 4C 遗留清理

> **⚠️ 本任务必须处理的两项（Task 3 审查留痕，2026-09-14）**：
> 1. **窗口内的层级序要统一裁决并写死在自有层注释里**。现状是「生成层给一套、自有层又抬了几个」，且 Task 3 只做了最小修复（`.mm-drawer { z-index: 35 }`）。已知的相对关系与归属：
>    - 顶栏 `.mm-topbar` 30 / 工具栏 `.mm-toolbar` 30（自有层，Task 2 为脱离画布抬的）
>    - 触发条 28 / 字数 25 / 导航栏 20 / 小地图 20（自有层）
>    - 抽屉 `.mm-drawer` **35**（自有层，Task 3 修 `✕` 被盖死时加）
>    - 大纲全屏编辑 40 → 拖拽遮罩 50 → 右键菜单 60 → 各弹窗 2000/3000 → toast 10001
>    - **待裁决**：两个搜索浮层（生成层 `.mm-search-box` **12**、画布内替换浮层 `.mm-search` **30**）在抽屉抬到 35 之后，由「压抽屉」翻成「**被抽屉压**」——原型是「搜索压抽屉」（12 > 10）。要么把两者抬到 35 之上（注意别撞大纲编辑 40）、要么明确接受翻转并写进差异表，二选一，**不要留默认状态**。
> 2. 本任务的「删被生成层取代的规则」清单里，**旧抽屉壳 `.mm-sidebar*`（`mindmap.css:241-286` 一带）已确认是死规则**（Task 3 把 `.mm-sidebar` 的用法整体换成了 `.mm-drawer`），要一并删；但注意 Task 3 只改了过渡名（`mm-drawer-slide`），整段壳样式仍在。
>
> 3. **三条 Task 4 审查留下的小项**（都属打磨，一并做并留注释）：
>    - 公式表：给 `<table>` 加 **`role="presentation"`**（现有 `tr` 上的 `role="button"` 会覆盖 `row`、使表格的 required-owned-elements 不成立；让掉隐式的 row/cell 角色后整块成为「一串按钮」，结构合法、零 CSS）。**务必留注释说明为什么不把表格语义改回来**——那份「表格」只剩单列、无 `<th>`/`<caption>`，行语义信息量≈0，而按钮语义（Tab / Enter / `aria-disabled` / 全局焦点环）是真收益。
>    - `.mm-icon-tile:disabled` 只改 `opacity`/`cursor`，生成层 `:hover` 仍给背景/边框/scale（`components.css:1123-1127`）⇒ 禁用态看起来仍可点；补齐复位或注明该分支实际不可达。
>    - `.mm-struct-groupname`（`mindmap.css:613`）已被三处当通用分组小标题复用，名字仍带 `struct` ⇒ 改名为通用名（如 `.mm-group-title`）并同步三处。

**Files:**
- Modify: `src/styles/mindmap.css`（删被生成层取代的规则、**统一层级序**）
- Modify: `src/windows/Main/LauncherPageView.vue:84`、`src/composables/useLauncherSettings.ts:83`（4C 验收记录 §5.1 的两处「失败仍报成功」）

**Interfaces:**
- Consumes：Task 2–5 引入的生成层类名
- Produces：`mindmap.css` 只含「真实窗口适配 + 项目扩展 + 生成层未覆盖的新组件」

- [ ] **Step 1: 删被生成层取代的规则**

按类名逐段核对，删掉这些**自造、现已被生成层取代**的块（保留注释说明去向）：

```
.mm-toolbar  → 生成层 .mm-topbar
.mm-trigger  → 生成层 .mm-sidebar-dock / .mm-dock-item
.mm-sidebar  → 生成层 .mm-drawer*
.mm-ctxmenu  → 生成层 .mm-ctx*（令牌已在 Task 5 改为 --mm-*）
.mm-count    → 生成层 .mm-bottombar 内的 .mm-stat-left
.mm-nav      → 生成层 .mm-bottombar 内的 .mm-ctrl-right
```

**保留**：`.mindmap-window`（无边框窗口的圆角/拖拽区）、`.mm-stage` / `.mm-canvas`、`.mm-panel`（项目扩展的玻璃质感）、`.mm-field` / `.mm-dialog`（我们的表单件）、`.mm-search`（我们的搜索行为）、禅模式与拖拽导入遮罩。

- [ ] **Step 2: 清理 4C 遗留**

`LauncherPageView.vue:84` 的 `persist` 与 `useLauncherSettings.ts:83` 的 `saveLauncherRoots`——两处都在 `patch` 失败后仍走成功路径（弹成功提示 / 继续重建索引）。按 4C `bd38a11` 修 `setHistoryRetention` 的同一手法处理：判断 `await patch(...)` 的返回值，失败即 `return` 并给一条失败 toast。

- [ ] **Step 3: 全链校验**

```bash
pnpm sync:styles --check && pnpm typecheck && pnpm test && pnpm format:check && pnpm exec vite build 2>&1 | grep -E "built in|error"
```

Expected: 全绿

- [ ] **Step 4: 提交**

```bash
git add -A src/styles/mindmap.css src/windows/Main/LauncherPageView.vue src/composables/useLauncherSettings.ts
git commit -m "refactor(mindmap): 自有层样式收敛为窗口适配与项目扩展；清理 4C 遗留的失败报成功"
```

---

## Task 8: 实机验收与记录

**Files:**
- Create: `docs/superpowers/plans/2026-09-14-prototype-realign-phase5-acceptance.md`
- Modify: spec §9（`docs/superpowers/specs/2026-09-14-prototype-realign-phase5-mindmap-design.md` 末节）

- [ ] **Step 1: 静态检查全绿**

Run: `pnpm sync:styles --check && pnpm typecheck && pnpm test && pnpm format:check && (cd src-tauri && cargo fmt --check && cargo check && cargo test) && pnpm exec vite build`

Expected: 全绿；记录前端/后端通过数与 `mindmap-*.js` chunk 体积

- [ ] **Step 2: 实机（`pnpm tauri:dev`；**先向用户请示是否接管键鼠**，按记忆「用户在本机时禁用键鼠自动化」）**按 spec §6 八项逐条验证并截图**

要点：① 顶栏三岛与改名 ② dock 六项与单壳切换 ③ 底栏 12 控件 ④ 画布右下角右键不被裁剪 ⑤ 打字机/深色/浅色三套主题下 chrome 可读且浅色不再暗底 ⑥ 能力回归（格式刷/关联线/外框/设置/小地图/搜索/导入/导出）⑦ 未保存提示 ⑧ `sync:styles --check` 通过

- [ ] **Step 3: 写验收记录并回填 spec §9**

结构照 `2026-09-13-prototype-realign-phase4c-acceptance.md`：环境与方法 → 自动化校验表 → 逐项比对表 → 发现的问题与补丁清单 → 未验证项 → 结论。spec §9 四条「待填」改为实际结论。

- [ ] **Step 4: 提交**

```bash
git add docs/superpowers/plans/2026-09-14-prototype-realign-phase5-acceptance.md docs/superpowers/specs/2026-09-14-prototype-realign-phase5-mindmap-design.md
git commit -m "docs(design): 阶段五实机验收记录"
```

---

## 自检记录

- **Spec 覆盖**：§3 差异表 #1/#2/#3/#4/#5 → Task 2；#6/#7/#11 → Task 3；#8/#9/#17 → Task 4；#12/#13 → Task 5；#14 → Task 5 Step 2；#15/#16 → Task 7 Step 1（保留我们的实现，只换样式）；#18/#19/#20 → Task 6；#21–#27 → Task 7 Step 1 的「保留」清单 + Task 8 第 6 项回归；D41 → Task 2（导出保留）；D42 → Task 4；D43 → Task 5（快捷键仍在「更多」）；D44 → Task 1/2；D45 → Task 7；D46 → Task 6；D47 → 全局；D48 → Task 2（`close` 不变）；D49 → 全局（不改 key 名）。§5 测试 → Task 1 与 Task 6 各一条纯函数/静态单测；§6 验收 → Task 8；§7 七个提交批次 ↔ Task 1–8；§8 风险 → 各 Task 的校验步骤。
- **占位扫描**：无 TBD / 「适当处理」；两处「实现时按现状确认」已收窄为「读 `core/persistence.ts` 的 `parseMindMapData` 默认值对齐 `setData` 入参」与「`iconfont` 类名必须取自 `src/assets/mindmap/icon-font/iconfont.css` 实有清单（计划里列的五个右岛图标已逐个核实，左岛沿用 `Toolbar.vue` 现有类名）」。
- **类型与命名一致性**：`normalizeMapName` / `MAP_NAME_LIMIT` / `MAP_NAME_FALLBACK` 在 Task 1 定义、Task 2 使用；`MmFilenameIsland` 的 props `name` 与 emit `rename` 在 Task 1 定义、Task 2 接线；`MmModal` 的 props `{ title, visible }` 在 Task 4 Step 1 定义、Step 2/3 使用；`MmDrawer` 只用 `ui.activeSidebar` 与 `sidebarTriggerList`（两者均为既有导出）。
