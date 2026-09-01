<script setup lang="ts">
import { baseKeymap, chainCommands, exitCode, toggleMark } from 'prosemirror-commands'
import { dropCursor } from 'prosemirror-dropcursor'
import { gapCursor } from 'prosemirror-gapcursor'
import { history, redo, undo } from 'prosemirror-history'
import { keymap } from 'prosemirror-keymap'
import { EditorState } from 'prosemirror-state'
import { EditorView } from 'prosemirror-view'
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { logger } from '@/service/logger'
import { codeBlockNodeView } from './codeblock'
import { buildInputRules } from './inputrules'
import { parseMarkdown, serializeMarkdown } from './markdown'
import { placeholderPlugin } from './placeholder'
import { marks, noteSchema } from './schema'

/**
 * 笔记编辑器：ProseMirror 所见即所得 + 代码块内嵌 CodeMirror。
 *
 * 需求 2.2：Markdown 即时渲染，敲完语法标记立刻变成渲染结果。
 * 正文以 Markdown 原文与外部通信（v-model），便于归档卡片直接渲染与导出。
 */
const props = withDefaults(
  defineProps<{
    modelValue: string
    placeholder?: string
  }>(),
  { placeholder: '此刻在想什么？直接写下来… 支持 **Markdown** 即时渲染，右下角管理标签' },
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
  /** 用户按下归档快捷键（⌘/⌃+Enter）。 */
  (e: 'submit'): void
}>()

const host = ref<HTMLElement | null>(null)
let view: EditorView | null = null
/** 正在把编辑器内容同步给外部，期间忽略 props 回灌，避免光标跳动。 */
let syncing = false

/** 快捷键：加粗/斜体/行内代码/删除线/撤销重做/归档。 */
function buildKeymap() {
  return keymap({
    'Mod-b': toggleMark(marks.strong),
    'Mod-i': toggleMark(marks.em),
    'Mod-e': toggleMark(marks.code),
    'Mod-Shift-x': toggleMark(marks.strikethrough),
    'Mod-z': undo,
    'Mod-y': redo,
    'Mod-Shift-z': redo,
    // ⌘/⌃+Enter：在代码块内先尝试跳出，否则视为「归档念头」
    'Mod-Enter': chainCommands(exitCode, () => {
      emit('submit')
      return true
    }),
  })
}

function createState(markdownSource: string): EditorState {
  return EditorState.create({
    doc: parseMarkdown(markdownSource),
    schema: noteSchema,
    plugins: [
      buildInputRules(),
      buildKeymap(),
      keymap(baseKeymap),
      history(),
      dropCursor(),
      gapCursor(),
      placeholderPlugin(props.placeholder),
    ],
  })
}

onMounted(() => {
  if (!host.value) return

  view = new EditorView(host.value, {
    state: createState(props.modelValue),
    // 代码块交给 CodeMirror 渲染
    nodeViews: { code_block: codeBlockNodeView },
    attributes: {
      class: 'editor ProseMirror',
      'data-placeholder': props.placeholder,
    },
    dispatchTransaction(transaction) {
      if (!view) return
      const next = view.state.apply(transaction)
      view.updateState(next)

      if (!transaction.docChanged) return
      syncing = true
      try {
        emit('update:modelValue', serializeMarkdown(next.doc))
      } finally {
        syncing = false
      }
    },
  })

  logger.info('note-editor', '编辑器已挂载')
})

// 外部内容变化（如切换笔记）时重建文档；自身编辑触发的回灌直接跳过。
watch(
  () => props.modelValue,
  (next) => {
    if (syncing || !view) return
    if (serializeMarkdown(view.state.doc) === next) return
    logger.debug('note-editor', '外部内容变更，重建文档')
    view.updateState(createState(next))
  },
)

onBeforeUnmount(() => {
  view?.destroy()
  view = null
  logger.info('note-editor', '编辑器已销毁')
})

/** 供父组件调用：聚焦编辑器。 */
function focus(): void {
  view?.focus()
}

defineExpose({ focus })
</script>

<template>
  <!-- ProseMirror 会把 .editor 类挂到它自己创建的可编辑节点上 -->
  <div ref="host" class="editor-host" />
</template>

<style scoped>
.editor-host {
  display: contents;
}
</style>
