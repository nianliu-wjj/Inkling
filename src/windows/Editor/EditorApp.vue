<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import ToastHost from '@/components/base/ToastHost.vue'
import TodoEditorPanel from '@/components/todo/TodoEditorPanel.vue'
import { useSettings, useTodos } from '@/composables/useData'
import type { EditorAnchor, TodoEditorPayload } from '@/composables/useEditorWindow'
import { applyCachedGlass, useGlass } from '@/composables/useGlass'
import { applyCachedTheme, useTheme } from '@/composables/useTheme'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { Todo, TodoInput } from '@/typings/domain'
import type { Rect } from '@/utils/anchor'

/**
 * 独立编辑窗口：铺满工作区的透明置顶窗，内含锚定卡片右侧的待办编辑浮层（spec D23）。
 *
 * 面板只有 480px 宽、主窗口内锚定会被窗口边界裁切，因此主窗口与面板都走这里；
 * payload 带锚点卡片的**物理屏幕像素**矩形，本窗按自己的 innerPosition / scaleFactor 换算回 CSS 像素后
 * 交给 TodoEditorPanel 按原型 positionTodoEditor 定位（右侧 → 左翻 → 居中兜底）。
 *
 * 打开参数在挂载时主动向后端拉取（见 app::windows::editor_open）。
 * 不用「常驻窗口 + 事件推参数」：WebView2 在窗口 hide 后会被挂起，
 * Tauri 靠 eval 投递的事件此时全部丢失，第二次打开只会得到一个空的全屏透明窗口；
 * 也不走 URL 查询串：`WebviewUrl::App` 收的是相对路径，`?` 会被转义掉。
 * 窗口每次打开都是新建的，挂载逻辑必然执行一次，拉取的时序是确定的。
 *
 * 保存直接走 IPC，后端会广播 todosChanged，面板与归档窗自行刷新，
 * 因此不需要把结果回传给调用方窗口。
 */

// 启动瞬间先用缓存主题上色，避免默认深色闪一下再跳变。
applyCachedTheme()
applyCachedGlass()

document.documentElement.dataset.window = 'editor'

const { todos } = useTodos()
const { settings } = useSettings()
const { applyTheme } = useTheme()
const { applyGlass } = useGlass()
const { toast } = useToast()

/** 本次打开参数，挂载后由后端拉取填入。 */
const payload = ref<TodoEditorPayload | null>(null)
/** 锚点换算结果（本窗 CSS 像素）；null = 无锚点居中。 */
const anchorRect = ref<Rect | null>(null)
/** 锚点是否已换算完成（无锚点时立即为 true）；未完成前不渲染，避免浮层先居中再跳到卡片旁。 */
const anchorResolved = ref(false)

/** 是否已请求显示窗口，避免 ready 反复变化时重复 invoke。 */
const shown = ref(false)

watch(
  () => settings.value.theme,
  (theme) => applyTheme(theme),
  { immediate: true },
)

// 玻璃质感与主题同源：后端设置变化时一并同步。
watch(
  () => settings.value.glass_level,
  (level) => applyGlass(level),
  { immediate: true },
)

/** 待编辑的事项：按 ID 从最新列表中取，保证拿到的是当前数据而非调用方的快照。 */
const todo = computed<Todo | null>(() => {
  const id = payload.value?.todoId
  if (!id) return null
  return todos.value.find((item) => item.id === id) ?? null
})

/** 父待办：新建子任务取 payload.parentId；编辑既有子任务取 todo.parent_id（完成时间上限校验用）。 */
const parentId = computed<string | null>(() => payload.value?.parentId ?? todo.value?.parent_id ?? null)
const parent = computed<Todo | null>(() => {
  const id = parentId.value
  if (!id) return null
  return todos.value.find((item) => item.id === id) ?? null
})

/** 参数、锚点与目标事项都就绪才渲染，避免闪出一张空表单或位置跳变。 */
const ready = computed(() => {
  const value = payload.value
  if (!value || !anchorResolved.value) return false
  if (value.todoId && !todo.value) return false
  if (parentId.value && !parent.value) return false
  return true
})

async function close(): Promise<void> {
  try {
    await api.windows.editorClose()
  } catch (error) {
    logger.error('editor', '关闭编辑窗口失败', error)
  }
}

/** 物理屏幕像素 → 本窗 CSS 像素（与 useEditorWindow.physicalAnchorOf 互逆）。 */
async function resolveAnchor(anchor: EditorAnchor | undefined): Promise<void> {
  if (!anchor) {
    anchorResolved.value = true
    return
  }
  try {
    const win = getCurrentWindow()
    const [position, scale] = await Promise.all([win.innerPosition(), win.scaleFactor()])
    anchorRect.value = {
      left: (anchor.x - position.x) / scale,
      top: (anchor.y - position.y) / scale,
      width: anchor.w / scale,
      height: anchor.h / scale,
    }
    logger.info('editor', '锚点换算完成', anchorRect.value)
  } catch (error) {
    logger.warn('editor', '锚点换算失败，浮层居中', error)
  }
  anchorResolved.value = true
}

/** 编辑浮层实例：保存失败时调用它暴露的 resetSaving() 解除在途守卫。 */
const panelRef = ref<InstanceType<typeof TodoEditorPanel> | null>(null)

async function saveTodo(input: TodoInput): Promise<void> {
  try {
    await api.todos.save(input)
    logger.info('editor', '待办已保存')
  } catch (error) {
    // 保存失败时保留窗口与已填内容，让用户能修正后重试；同时解除面板的保存在途守卫，否则再点保存会被吞掉。
    // 主窗口的待办保存也走本窗，后端拒绝（如「最多 5 个子任务」）必须在这里 toast 出来，否则用户只看到保存无反应。
    logger.error('editor', '保存待办失败', error)
    toast(String(error))
    panelRef.value?.resetSaving()
    return
  }
  void close()
}

/**
 * 内容首次可渲染后再请求显示窗口：窗口以 visible(false) 创建，
 * 否则会先闪出一片空遮罩，等数据到位才出现对话框。
 */
watch(
  ready,
  (value) => {
    if (!value || shown.value) return
    shown.value = true
    void nextTick(() => {
      void api.windows.editorReady().catch((error) => logger.error('editor', '显示编辑窗口失败', error))
    })
  },
  { immediate: true },
)

// 编辑中的待办 / 父待办被别处删除时不能留下一个吞点击的透明置顶窗：ready 由 true 翻回 false 即自毁。
watch(ready, (now, prev) => {
  if (prev && !now) {
    logger.warn('editor', '目标待办或父待办已不存在，关闭窗口')
    void close()
  }
})

// 立刻取参数，不等挂载：窗口是为这次打开新建的，越早拿到越早能渲染出对话框。
void api.windows
  .editorPayload()
  .then((raw) => {
    if (!raw) throw new Error('后端未提供打开参数')
    payload.value = JSON.parse(raw) as TodoEditorPayload
    logger.info('editor', '打开参数', payload.value)
    return resolveAnchor(payload.value.anchor)
  })
  .catch((error) => {
    // 参数缺失或损坏时不能留一个吞掉整屏点击的透明窗口，直接自毁。
    logger.error('editor', '获取打开参数失败，关闭窗口', error)
    void close()
  })

onMounted(() => logger.info('editor', '编辑窗口已挂载'))
</script>

<template>
  <TodoEditorPanel
    v-if="ready && payload?.kind === 'todo'"
    ref="panelRef"
    :mode="payload.mode"
    :todo="todo"
    :parent="parent"
    :preset-date="payload.presetDate ?? ''"
    :anchor="anchorRect"
    @save="saveTodo"
    @close="close"
  />

  <ToastHost />
</template>
