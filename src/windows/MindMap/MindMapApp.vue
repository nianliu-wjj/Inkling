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
 * 顶部是 Inkling 操作条（标题 / 未保存标记 / 标签 / 关闭 / 保存），下方是铺满的画布区，
 * 编辑 UI（工具栏 / 侧栏 / 浮层 / 对话框）由后续阶段挂进 `.mm-stage`。
 *
 * 本组件在根部用 provide 注入 { mindMap, bus, ui, localConfig, mapConfig }，
 * 并负责：库事件 → bus、持久化（全量格式）、自动保存、本机配置落盘与同步到库。
 */
applyCachedTheme()
applyCachedGlass()
document.documentElement.dataset.window = 'mindmap'

const label = getCurrentWindow().label

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
const title = computed(() => (isNew.value ? '新建思维导图' : '编辑思维导图'))
const note = computed(() => notes.value.find((item) => item.id === noteId.value) ?? null)
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
      content: note.value?.content ?? '',
      tags: [...tags.value],
      editorMode: 'mindmap',
      mindmapData: mindmapData.value,
      draft: false,
    })
    noteId.value = saved.id
    dirty.value = false
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
          <span class="mindmap-title">🧠 {{ title }}<em v-if="dirty" class="mindmap-dirty">未保存</em></span>
          <div class="mindmap-actions">
            <div class="tag-preview" title="点击管理标签">
              <TagList :tags="tags" :max="3" @open="showTagManager = true" />
            </div>
            <button type="button" class="btn" @click="close">关闭</button>
            <button type="button" class="btn primary" @click="save()">保存 ⌃S</button>
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
            <Toolbar v-if="!ui.isZenMode" />
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
