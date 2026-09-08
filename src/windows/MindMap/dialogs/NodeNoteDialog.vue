<script setup lang="ts">
import { NModal } from 'naive-ui'
import { onBeforeUnmount, onMounted, ref } from 'vue'
import type { MindMapNode } from 'simple-mind-map'
import { useToast } from '@/composables/useToast'
import { useMindMap } from '../core/useMindMap'

/** 节点备注对话框（移植参考端 NodeNote.vue）：多行文本 → `node.setNote(note)`。 */
const { bus, ui } = useMindMap()
const { toast } = useToast()

const show = ref(false)
const node = ref<MindMapNode | null>(null)
const note = ref('')

function open(): void {
  node.value = ui.activeNodes[0] ?? null
  if (!node.value) {
    toast('请先选择一个节点')
    return
  }
  note.value = node.value.getData('note') || ''
  show.value = true
}

function confirm(): void {
  node.value?.setNote(note.value)
  show.value = false
}

const offs: Array<() => void> = []
onMounted(() => offs.push(bus.on('showNodeNote', open)))
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <NModal v-model:show="show">
    <div class="mm-dialog mm-panel">
      <div class="mm-dialog-title">备注</div>
      <textarea v-model="note" class="mm-dialog-textarea" placeholder="给节点添加备注…" />
      <div class="mm-dialog-actions">
        <button type="button" class="btn" @click="show = false">取消</button>
        <button type="button" class="btn primary" @click="confirm">确定</button>
      </div>
    </div>
  </NModal>
</template>
