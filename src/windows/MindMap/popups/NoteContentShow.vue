<script setup lang="ts">
import { onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { renderMarkdown } from '@/utils/format'
import { useMindMap } from '../core/useMindMap'

/**
 * 节点备注悬浮展示（移植参考端 NodeNoteContentShow.vue）。
 *
 * 库通过 createMindMap 的 customNoteContentShow 回调发出 `showNoteContent(content,left,top,node)`
 * 与 `hideNoteContent`：悬停节点备注图标时在指定位置渲染备注 Markdown，移开隐藏。
 * 双击备注图标（库事件 node_note_dblclick）复用既有备注对话框（发 showNodeNote）。
 */
const { bus } = useMindMap()

const show = ref(false)
const style = reactive<{ left: string; top: string }>({ left: '0', top: '0' })
const html = ref('')

function onShow(content: string, left: number, top: number): void {
  html.value = renderMarkdown(content || '')
  style.left = left + 'px'
  style.top = top + 'px'
  show.value = true
}
function onHide(): void {
  show.value = false
}
function onNoteDblclick(): void {
  bus.emit('showNodeNote')
}

const offs: Array<() => void> = []
onMounted(() => {
  offs.push(
    bus.on('showNoteContent', (content, left, top) => onShow(content as string, left as number, top as number)),
    bus.on('hideNoteContent', onHide),
    bus.on('node_note_dblclick', onNoteDblclick),
  )
})
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <Teleport to="body">
    <div v-show="show" class="mm-note-show mm-panel" :style="style" v-html="html" />
  </Teleport>
</template>
