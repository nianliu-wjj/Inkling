<script setup lang="ts">
import { NModal } from 'naive-ui'
import { onBeforeUnmount, onMounted, ref } from 'vue'
import type { MindMapNode } from 'simple-mind-map'
import { useToast } from '@/composables/useToast'
import { useMindMap } from '../core/useMindMap'

/** 节点超链接对话框：`node.setHyperlink(link, title)`；打开回显现有值。 */
const { bus, ui } = useMindMap()
const { toast } = useToast()

const show = ref(false)
const node = ref<MindMapNode | null>(null)
const link = ref('')
const title = ref('')

function open(): void {
  node.value = ui.activeNodes[0] ?? null
  if (!node.value) {
    toast('请先选择一个节点')
    return
  }
  link.value = node.value.getData('hyperlink') || ''
  title.value = node.value.getData('hyperlinkTitle') || ''
  show.value = true
}

function confirm(): void {
  node.value?.setHyperlink(link.value.trim(), title.value.trim())
  show.value = false
}

const offs: Array<() => void> = []
onMounted(() => offs.push(bus.on('showNodeLink', open)))
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <NModal v-model:show="show">
    <div class="mm-dialog mm-panel">
      <div class="mm-dialog-title">超链接</div>
      <label class="mm-field">
        <span class="mm-field-label">链接</span>
        <input v-model="link" class="mm-dialog-input" placeholder="https://…" />
      </label>
      <label class="mm-field">
        <span class="mm-field-label">名称</span>
        <input v-model="title" class="mm-dialog-input" placeholder="可留空" />
      </label>
      <div class="mm-dialog-actions">
        <button type="button" class="btn" @click="show = false">取消</button>
        <button type="button" class="btn primary" @click="confirm">确定</button>
      </div>
    </div>
  </NModal>
</template>
