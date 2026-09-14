<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { NConfigProvider, NDialogProvider, dateZhCN, zhCN } from 'naive-ui'
import type MindMap from 'simple-mind-map'
import type { MindMapNode } from 'simple-mind-map'
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, shallowRef, watch } from 'vue'
import ToastHost from '@/components/base/ToastHost.vue'
import TagList from '@/components/tag/TagList.vue'
import TagManagerModal from '@/components/tag/TagManagerModal.vue'
import { useNotes, useSettings } from '@/composables/useData'
import { applyCachedGlass, useGlass } from '@/composables/useGlass'
import { applyCachedTheme, useTheme } from '@/composables/useTheme'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import { MAP_NAME_FALLBACK } from '@/utils/mindmapName'
import { createBus } from './core/bus'
import { toggleRichText, toggleScrollbar } from './core/createMindMap'
import { loadLocalConfig, loadMapConfig, saveLocalConfig, saveMapConfig } from './core/localConfig'
import { buildThemeOverrides, naiveDark } from './core/naiveTheme'
import { type MindMapFullData, parseMindMapData, serializeMindMapData } from './core/persistence'
import { createUiState } from './core/store'
import { provideMindMapContext } from './core/useMindMap'
import Toolbar from './chrome/Toolbar.vue'
import NavigatorToolbar from './chrome/NavigatorToolbar.vue'
import Navigator from './chrome/Navigator.vue'
import ScrollbarBars from './chrome/ScrollbarBars.vue'
import SidebarTrigger from './chrome/SidebarTrigger.vue'
import Count from './chrome/Count.vue'
import ContextMenu from './popups/ContextMenu.vue'
import SearchBox from './popups/SearchBox.vue'
import RichTextToolbar from './popups/RichTextToolbar.vue'
import NodeIconToolbar from './popups/NodeIconToolbar.vue'
import NodeImgPlacementToolbar from './popups/NodeImgPlacementToolbar.vue'
import NoteContentShow from './popups/NoteContentShow.vue'
import NodeImgPreview from './popups/NodeImgPreview.vue'
import OuterFramePanel from './popups/OuterFramePanel.vue'
import TagStylePanel from './popups/TagStylePanel.vue'
import AssociativeLineStylePanel from './popups/AssociativeLineStylePanel.vue'
import ShortcutSidebar from './sidebars/ShortcutSidebar.vue'
import StructureSidebar from './sidebars/StructureSidebar.vue'
import ThemeSidebar from './sidebars/ThemeSidebar.vue'
import BaseStyleSidebar from './sidebars/BaseStyleSidebar.vue'
import NodeStyleSidebar from './sidebars/NodeStyleSidebar.vue'
import SettingSidebar from './sidebars/SettingSidebar.vue'
import IconSidebar from './sidebars/IconSidebar.vue'
import FormulaSidebar from './sidebars/FormulaSidebar.vue'
import NoteSidebar from './sidebars/NoteSidebar.vue'
import OutlineSidebar from './sidebars/OutlineSidebar.vue'
import ImportDialog from './dialogs/ImportDialog.vue'
import ExportDialog from './dialogs/ExportDialog.vue'
import NodeImageDialog from './dialogs/NodeImageDialog.vue'
import NodeHyperlinkDialog from './dialogs/NodeHyperlinkDialog.vue'
import NodeNoteDialog from './dialogs/NodeNoteDialog.vue'
import NodeTagDialog from './dialogs/NodeTagDialog.vue'
import OutlineEditDialog from './dialogs/OutlineEditDialog.vue'
import SourceCodeDialog from './dialogs/SourceCodeDialog.vue'
import NodeLinkDialog from './dialogs/NodeLinkDialog.vue'
import { setupAttachmentPicker } from './dialogs/AttachmentPicker'
import MindMapStage from './MindMapStage.vue'

/**
 * 思维导图窗口壳。
 *
 * 独立顶层窗口，一个笔记一个（label 形如 `mindmap-<id>`，新建用 `mindmap-new`）。
 * 顶部是 Inkling 操作条（导图名 / 未保存标记 / 标签；保存与关闭已随阶段五迁进顶栏右岛），
 * 下方是铺满的画布区，编辑 UI（顶栏三岛 / 侧栏 / 浮层 / 对话框）挂在 `.mm-stage` 内。
 *
 * 本组件在根部用 provide 注入 { mindMap, bus, ui, localConfig, mapConfig }，
 * 并负责：库事件 → bus、持久化（全量格式）、自动保存、本机配置落盘与同步到库。
 */
applyCachedTheme()
applyCachedGlass()
document.documentElement.dataset.window = 'mindmap'

const label = getCurrentWindow().label
/** 新建导图的窗口 label，与 `windows.rs::mindmap_open` 里 `note_id == None` 分支保持一致。 */
const NEW_MAP_LABEL = 'mindmap-new'

const { notes } = useNotes()
const { settings } = useSettings()
const { applyTheme } = useTheme()
const { applyGlass } = useGlass()
const { toast } = useToast()

// —— 共享上下文 ——
const bus = createBus()
const ui = createUiState()
const localConfig = reactive(loadLocalConfig())
const mapConfig = reactive(loadMapConfig())
const mindMap = shallowRef<MindMap | null>(null)
provideMindMapContext({ mindMap, bus, ui, localConfig, mapConfig })

// —— 窗口状态 ——
const noteId = ref('')
/** 打开参数是否已拉取完成。 */
const payloadReady = ref(false)
const mindmapData = ref<string | null>(null)
const tags = ref<string[]>([])
const showTagManager = ref(false)
const dirty = ref(false)
const themeOverrides = ref(buildThemeOverrides())

const isNew = computed(() => !noteId.value)
const note = computed(() => notes.value.find((item) => item.id === noteId.value) ?? null)
/** 未保存的新导图上改的名字（此时还没有笔记可写，先记在本地，手动保存时一并写进 `content`）。 */
const pendingName = ref('')
/**
 * 顶栏文件名岛与窗口标题显示的名字。
 *
 * 导图笔记的 `content` 就是导图名，所以已有笔记直接取它；尚未保存的新导图取暂存名，
 * 两者都没有时回退原型同款文案。注意不能为了改名就给新导图静默建一条笔记——
 * 那会破坏 `markDirtyAndAutosave` 里「新建导图首次必须手动保存」的约定。
 */
const mapName = computed(() => note.value?.content || pendingName.value || MAP_NAME_FALLBACK)
/**
 * 落库用的导图名：优先用户刚改的暂存名，其次笔记原标题；**不含显示回退**。
 *
 * `mapName` 的 `MAP_NAME_FALLBACK` 只为显示（顶栏文件名岛 / 窗口标题），不能兼作落库默认值——
 * 既有笔记的 `content` 本来就是空串时，保存一次会把标题写成「未命名导图」。
 */
const persistName = computed(() => pendingName.value || note.value?.content || '')
/**
 * 何时可以创建画布：参数已就绪，且——新建导图立即可建，编辑既有导图必须等目标笔记从
 * 列表异步加载出来。否则会先用空的「中心主题」建实例，等真实数据到达时画布已经建好、
 * 不再更新，用户看到的是一张空导图（这正是首次实现时踩到的坑）。
 */
const canRender = computed(() => payloadReady.value && (isNew.value || note.value !== null))
/** 初始全量数据：编辑既有导图取其存储数据，新建则为空根节点。画布只在 canRender 后渲染一次。 */
const initialData = computed<MindMapFullData>(() => parseMindMapData(note.value?.mindmap_data ?? null))

watch(
  () => settings.value.theme,
  (theme) => {
    applyTheme(theme)
    void nextTick(() => (themeOverrides.value = buildThemeOverrides()))
  },
  { immediate: true },
)
watch(
  () => settings.value.glass_level,
  (level) => applyGlass(level),
  { immediate: true },
)

// 本机配置改动：落盘 + 同步到库
watch(localConfig, (next) => saveLocalConfig({ ...next }), { deep: true })
watch(
  () => localConfig.openNodeRichText,
  (v) => mindMap.value && toggleRichText(mindMap.value, v),
)
watch(
  () => localConfig.isShowScrollbar,
  (v) => mindMap.value && toggleScrollbar(mindMap.value, v),
)
watch(
  () => localConfig.useLeftKeySelectionRightKeyDrag,
  (v) => mindMap.value?.updateConfig({ useLeftKeySelectionRightKeyDrag: v }),
)
watch(
  () => localConfig.isZenMode,
  (v) => {
    ui.isZenMode = v
  },
  { immediate: true },
)
watch(
  mapConfig,
  (next) => {
    saveMapConfig({ ...next })
    mindMap.value?.updateConfig({ ...next })
  },
  { deep: true },
)

// —— 自动保存（spec D6）——
let autosaveTimer: ReturnType<typeof setTimeout> | null = null
/** 附件选择器的清理函数（onCreated 里安装，窗口卸载时调用）。 */
let detachAttachmentPicker: (() => void) | null = null

/** view_data_change 只改视图位置：仍保存（视图是全量数据一部分），但不标记「未保存」打扰用户。 */
function markDirtyAndAutosave(viewOnly = false): void {
  const instance = mindMap.value
  if (!instance) return
  mindmapData.value = serializeMindMapData(instance.getData(true) as MindMapFullData)
  if (!viewOnly) dirty.value = true
  if (!noteId.value) return // 新建导图首次必须手动保存，否则每开一次就静默产生空笔记
  if (autosaveTimer) clearTimeout(autosaveTimer)
  autosaveTimer = setTimeout(() => {
    autosaveTimer = null
    void save(true)
  }, 1500)
}

async function save(silent = false): Promise<void> {
  const instance = mindMap.value
  if (instance) {
    mindmapData.value = serializeMindMapData(instance.getData(true) as MindMapFullData)
  }
  if (!mindmapData.value) {
    if (!silent) toast('请先编辑思维导图内容')
    return
  }
  try {
    const saved = await api.notes.save({
      id: noteId.value || undefined,
      // 用 persistName 而不是 note.content：新建导图的暂存名（在文件名岛上改的）要随首次保存写进笔记；
      // 也不用 mapName：显示回退（「未命名导图」）不该被写进库，空的标题就保持为空。
      content: persistName.value,
      tags: [...tags.value],
      editorMode: 'mindmap',
      mindmapData: mindmapData.value,
      draft: false,
    })
    noteId.value = saved.id
    dirty.value = false
    // 暂存名已经写进笔记，不再需要。但笔记列表要靠 notes-changed 异步重拉，此刻 note 可能还是
    // null，直接清会让文件名岛闪一下「未命名导图」，故等笔记真的出现在列表里再清。
    if (notes.value.some((item) => item.id === saved.id)) pendingName.value = ''
    if (silent) logger.info('mindmap', `自动保存 id=${saved.id}`)
    else {
      toast('已保存')
      logger.info('mindmap', `保存思维导图 id=${saved.id}`)
    }
  } catch (error) {
    logger.error('mindmap', '保存思维导图失败', error)
    if (!silent) toast('保存失败')
  }
}

/**
 * 文件名岛改名。
 *
 * 已有笔记：立即写回 `content`（`save()` 写的就是这个字段），主窗口经 notes-changed 刷新列表。
 * 尚未保存的新导图：只记在本地（`pendingName`），等用户手动保存时随 `content` 一起落库——
 * 改名不是保存，不能顺手建一条笔记出来。
 */
async function renameTo(name: string): Promise<void> {
  if (!noteId.value) {
    pendingName.value = name
    logger.info('mindmap', `新导图暂存名「${name}」，首次保存时写入`)
    return
  }
  try {
    await api.notes.save({
      id: noteId.value,
      content: name,
      tags: [...tags.value],
      editorMode: 'mindmap',
      // 画布上可能还有未落库的改动：连当前数据一起写回，避免改名把导图数据回退成上一次保存的内容。
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

/**
 * 新建：原型 `#mmNew`（`docs/app.js:1748-1756`）——确认后换成一个空白导图。
 *
 * 必须「换窗口」而不是就地清空：窗口 label 是「一个笔记一个窗口」的唯一保证
 * （`windows.rs::mindmap_open` 按 label 复用，命中即聚焦、不叠第二个），而 label 在
 * Tauri v2 里不可变。就地清空会让本窗顶着旧笔记的 label 显示新导图 —— 那条笔记的
 * 导图在关窗之前打不开；新建保存后从主窗口再打开新笔记还会另开一窗，两窗同编一条
 * 笔记、自动保存互相覆盖。
 *
 * 先开新窗再关本窗：中途失败也只是多留一个窗口，不会两边都没有。
 *
 * 例外：本窗 label 已是 `mindmap-new`（用户在一个尚未保存的新导图上再点新建）时不能换窗 ——
 * `mindmap_open` 会命中本窗自身并原样返回，紧接着关掉的就是唯一那个窗口。此态下 label 与内容
 * 本来就一致（都表示「新建」），不存在错配，就地清空即可。
 */
function startNewMap(): void {
  if (!window.confirm('新建导图将清空当前未保存的内容，确认新建？')) return
  if (label === NEW_MAP_LABEL) {
    // label 已与内容一致，没有错配，就地重建为空导图（换窗前的老行为）。
    const instance = mindMap.value
    if (!instance) return
    pendingName.value = ''
    mindmapData.value = null
    // 入参形状与 `parseMindMapData` 的空数据兜底一致（`{ data: { text }, children: [] }`），
    // 库里 setData 要的是 root 节点，不是全量对象。setData 会重置历史记录，正合「新建」语义。
    instance.setData(parseMindMapData(null).root)
    instance.view.fit()
    dirty.value = false
    logger.info('mindmap', '已清空为新建导图（label 已是 mindmap-new，就地重建）')
    return
  }
  const closing = label
  logger.info('mindmap', `新建导图：切换窗口 ${closing} → ${NEW_MAP_LABEL}`)
  void api.windows
    .mindmapOpen()
    .then(() => api.windows.mindmapClose(closing))
    .catch((error: unknown) => logger.error('mindmap', '新建导图失败', error))
}

async function close(): Promise<void> {
  if (dirty.value && !window.confirm('思维导图尚未保存，确认关闭？')) return
  try {
    await api.windows.mindmapClose(label)
  } catch (error) {
    logger.error('mindmap', '关闭窗口失败', error)
  }
}

function saveTags(next: string[]): void {
  tags.value = next
  dirty.value = true
  showTagManager.value = false
}

// —— 拖拽文件导入（spec：enableDragImport 开启且非大纲树拖拽时）——
const dragImportActive = ref(false)

/** 是否允许当前拖拽触发导入（偏好开启，且不是在拖拽大纲树节点）。 */
function canDragImport(): boolean {
  return localConfig.enableDragImport && !ui.isDragOutlineTreeNode
}
function onDragEnter(): void {
  if (canDragImport()) dragImportActive.value = true
}
function onDragOver(): void {
  if (canDragImport()) dragImportActive.value = true
}
function onDragLeave(event: DragEvent): void {
  // 仅当离开舞台边界时收起遮罩（避免子元素间移动误触）。
  if (!(event.relatedTarget instanceof Node) || !(event.currentTarget as HTMLElement).contains(event.relatedTarget)) {
    dragImportActive.value = false
  }
}
function onDrop(event: DragEvent): void {
  dragImportActive.value = false
  if (!canDragImport()) return
  const file = event.dataTransfer?.files?.[0]
  if (file) bus.emit('importFile', file)
}

/** 画布实例创建完成：绑定库事件到 bus 与保存链路。 */
function onCreated(instance: MindMap): void {
  mindMap.value = instance
  bus.on('data_change', () => markDirtyAndAutosave())
  bus.on('view_data_change', () => markDirtyAndAutosave(true))
  bus.on('setData', (data) => {
    const full = data as Partial<MindMapFullData>
    if (full.root) instance.setFullData(full)
    else instance.setData(data)
    instance.view.reset()
    markDirtyAndAutosave()
  })
  bus.on('execCommand', (...args) => instance.execCommand(...(args as [string, ...unknown[]])))
  bus.on('export', async (...args) => {
    try {
      await instance.export(...(args as [string, boolean, string]))
    } catch (error) {
      logger.error('mindmap', '导出失败', error)
    }
  })
  bus.on('startPainter', () => instance.painter?.startPainter())
  bus.on('createAssociativeLine', () => instance.associativeLine?.createLineFromActiveNode())
  bus.on('startTextEdit', () => instance.renderer.startTextEdit())
  bus.on('endTextEdit', () => instance.renderer.endTextEdit())
  bus.on('node_active', (_node, list) => {
    ui.activeNodes = [...((list as MindMapNode[]) ?? [])]
  })
  bus.on('mode_change', (mode) => {
    ui.isReadonly = mode === 'readonly'
  })
  bus.on('toast', (text) => toast(String(text)))
  // 超链接跳转：`#uid` 定位到目标节点，否则用系统默认程序打开外链。
  bus.on('hyperlinkJump', (link) => {
    const value = String(link)
    if (value.startsWith('#')) instance.execCommand('GO_TARGET_NODE', value.slice(1))
    else void api.system.openUrl(value)
  })
  // 附件选择 / 打开 / 删除（非组件，返回清理函数在窗口卸载时调用）。
  detachAttachmentPicker = setupAttachmentPicker({ mindMap, bus, ui, localConfig, mapConfig })
  instance.keyCommand.addShortcut('Control+s', () => void save())
}

function onKeydown(event: KeyboardEvent): void {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 's') {
    event.preventDefault()
    void save()
  }
}

onMounted(() => {
  window.addEventListener('keydown', onKeydown)
  void api.windows
    .mindmapPayload(label)
    .then((id) => {
      noteId.value = id ?? ''
      payloadReady.value = true
      logger.info('mindmap', `打开思维导图 label=${label} id=${noteId.value || '(新建)'}`)
    })
    .catch((error) => {
      logger.error('mindmap', '获取打开参数失败', error)
      payloadReady.value = true
    })
})

// 目标笔记加载完成后填入导图数据与标签（新建时列表里没有它，保持空白）。
watch(note, (value) => {
  if (!value || dirty.value) return
  mindmapData.value = value.mindmap_data
  tags.value = [...value.tags]
})

onUnmounted(() => {
  window.removeEventListener('keydown', onKeydown)
  detachAttachmentPicker?.()
  if (autosaveTimer) clearTimeout(autosaveTimer)
})
</script>

<template>
  <NConfigProvider :theme="naiveDark" :theme-overrides="themeOverrides" :locale="zhCN" :date-locale="dateZhCN">
    <NDialogProvider>
      <div class="mindmap-window" :class="{ zen: ui.isZenMode }">
        <header class="mindmap-bar">
          <span class="mindmap-title">🧠 {{ mapName }}<em v-if="dirty" class="mindmap-dirty">未保存</em></span>
          <div class="mindmap-actions">
            <div class="tag-preview" title="点击管理标签">
              <TagList :tags="tags" :max="3" @open="showTagManager = true" />
            </div>
          </div>
        </header>

        <div
          class="mm-stage"
          @dragenter.prevent="onDragEnter"
          @dragover.prevent="onDragOver"
          @dragleave="onDragLeave"
          @drop.prevent="onDrop"
        >
          <MindMapStage v-if="canRender" :data="initialData" @created="onCreated" />
          <!-- 拖拽文件导入遮罩 -->
          <div v-if="dragImportActive" class="mm-drag-mask">松开鼠标导入该文件</div>
          <!-- 编辑 UI：仅在实例就绪后渲染，避免组件里 requireMindMap 抛错 -->
          <template v-if="mindMap">
            <Toolbar
              v-if="!ui.isZenMode"
              :map-name="mapName"
              @rename="renameTo"
              @save="save()"
              @close="close"
              @new-map="startNewMap"
            />
            <NavigatorToolbar v-if="!ui.isZenMode" />
            <Navigator />
            <ScrollbarBars />
            <SidebarTrigger v-if="!ui.isZenMode" />
            <Count v-if="!ui.isZenMode" />
            <ContextMenu />
            <SearchBox />
            <RichTextToolbar />
            <NodeIconToolbar />
            <NodeImgPlacementToolbar />
            <NoteContentShow />
            <NodeImgPreview />
            <OuterFramePanel />
            <TagStylePanel />
            <AssociativeLineStylePanel />
            <ShortcutSidebar />
            <StructureSidebar />
            <ThemeSidebar />
            <BaseStyleSidebar />
            <NodeStyleSidebar />
            <SettingSidebar />
            <IconSidebar />
            <FormulaSidebar />
            <NoteSidebar />
            <OutlineSidebar />
            <ImportDialog />
            <ExportDialog />
            <NodeImageDialog />
            <NodeHyperlinkDialog />
            <NodeNoteDialog />
            <NodeTagDialog />
            <OutlineEditDialog />
            <SourceCodeDialog />
            <NodeLinkDialog />
          </template>
          <!-- 后续阶段在此挂 NavigatorToolbar / 各侧栏 / 各浮层 / 各对话框 -->
        </div>

        <TagManagerModal
          v-if="showTagManager"
          :tags="tags"
          :max-length="5"
          subtitle="当前思维导图的标签"
          @save="saveTags"
          @close="showTagManager = false"
        />

        <ToastHost />
      </div>
    </NDialogProvider>
  </NConfigProvider>
</template>
