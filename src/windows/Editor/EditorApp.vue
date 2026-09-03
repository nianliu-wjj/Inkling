<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import ToastHost from '@/components/base/ToastHost.vue'
import TodoEditorModal from '@/components/todo/TodoEditorModal.vue'
import { useSettings, useTodos } from '@/composables/useData'
import { applyCachedTheme, useTheme } from '@/composables/useTheme'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { Todo, TodoInput } from '@/typings/domain'

/**
 * 独立编辑窗口：全屏压暗遮罩 + 居中对话框。
 *
 * 面板窗口只有 480px 宽、高度随内容伸缩（≤600px），编辑弹窗留在面板内必然被窗口
 * 边界裁切；独立成窗后对话框尺寸不再受面板约束（参考原型 doc/index.html 的模态设计）。
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

document.documentElement.dataset.window = 'editor'

/** 与 TodoPage 的 openEditor 参数一一对应。 */
type EditorPayload = {
  kind: 'todo'
  mode: 'create' | 'edit' | 'child'
  todoId?: string | null
  parentId?: string | null
  presetDate?: string
  focus?: 'content' | 'due' | 'remind'
}

const { todos } = useTodos()
const { settings } = useSettings()
const { applyTheme } = useTheme()

/** 本次打开参数，挂载后由后端拉取填入。 */
const payload = ref<EditorPayload | null>(null)

/** 是否已请求显示窗口，避免 ready 反复变化时重复 invoke。 */
const shown = ref(false)

watch(
  () => settings.value.theme,
  (theme) => applyTheme(theme),
  { immediate: true },
)

/** 待编辑的事项：按 ID 从最新列表中取，保证拿到的是当前数据而非调用方的快照。 */
const todo = computed<Todo | null>(() => {
  const id = payload.value?.todoId
  if (!id) return null
  return todos.value.find((item) => item.id === id) ?? null
})

/** 新建子任务时的父待办，用于完成时间上限校验。 */
const parent = computed<Todo | null>(() => {
  const id = payload.value?.parentId
  if (!id) return null
  return todos.value.find((item) => item.id === id) ?? null
})

/** 参数已就绪且目标事项已加载（编辑态）时才渲染，避免闪出一张空表单。 */
const ready = computed(() => {
  const value = payload.value
  if (!value) return false
  if (value.todoId && !todo.value) return false
  if (value.parentId && !parent.value) return false
  return true
})

async function close(): Promise<void> {
  try {
    await api.windows.editorClose()
  } catch (error) {
    logger.error('editor', '关闭编辑窗口失败', error)
  }
}

async function saveTodo(input: TodoInput): Promise<void> {
  try {
    await api.todos.save(input)
    logger.info('editor', '待办已保存')
  } catch (error) {
    // 保存失败时保留窗口与已填内容，让用户能修正后重试。
    logger.error('editor', '保存待办失败', error)
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

// 立刻取参数，不等挂载：窗口是为这次打开新建的，越早拿到越早能渲染出对话框。
void api.windows
  .editorPayload()
  .then((raw) => {
    if (!raw) throw new Error('后端未提供打开参数')
    payload.value = JSON.parse(raw) as EditorPayload
    logger.info('editor', '打开参数', payload.value)
  })
  .catch((error) => {
    // 参数缺失或损坏时不能留一个吞掉整屏点击的透明窗口，直接自毁。
    logger.error('editor', '获取打开参数失败，关闭窗口', error)
    void close()
  })

onMounted(() => logger.info('editor', '编辑窗口已挂载'))
</script>

<template>
  <TodoEditorModal
    v-if="ready && payload?.kind === 'todo'"
    :mode="payload.mode"
    :todo="todo"
    :parent="parent"
    :preset-date="payload.presetDate ?? ''"
    :focus="payload.focus ?? 'content'"
    @save="saveTodo"
    @close="close"
  />

  <ToastHost />
</template>
