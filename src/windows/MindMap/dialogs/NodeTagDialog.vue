<script setup lang="ts">
import { NModal } from 'naive-ui'
import { onBeforeUnmount, onMounted, ref } from 'vue'
import type { MindMapNode } from 'simple-mind-map'
import { useToast } from '@/composables/useToast'
import { requireMindMap, useMindMap } from '../core/useMindMap'

/**
 * 节点标签对话框（移植参考端 NodeTag.vue）：回车添加、点 ✕ 删除、最多 maxTag 个。
 * 标签存字符串数组，`node.setTag(list)`。
 */
const ctx = useMindMap()
const { bus, ui } = ctx
const { toast } = useToast()

const show = ref(false)
const node = ref<MindMapNode | null>(null)
const tags = ref<string[]>([])
const draft = ref('')

function open(): void {
  node.value = ui.activeNodes[0] ?? null
  if (!node.value) {
    toast('请先选择一个节点')
    return
  }
  const raw = node.value.getData('tag')
  // 标签可能是字符串或 { text } 对象，统一取文本。
  tags.value = Array.isArray(raw)
    ? raw.map((t: unknown) => (typeof t === 'string' ? t : ((t as { text?: string }).text ?? ''))).filter(Boolean)
    : []
  draft.value = ''
  show.value = true
}

function add(): void {
  const text = draft.value.trim()
  if (!text) return
  const maxTag = (requireMindMap(ctx).opt.maxTag as number) ?? 5
  if (tags.value.length >= maxTag) {
    toast(`最多 ${maxTag} 个标签`)
    return
  }
  if (!tags.value.includes(text)) tags.value.push(text)
  draft.value = ''
}

function remove(index: number): void {
  tags.value.splice(index, 1)
}

function confirm(): void {
  node.value?.setTag([...tags.value])
  show.value = false
}

const offs: Array<() => void> = []
onMounted(() => offs.push(bus.on('showNodeTag', open)))
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <NModal v-model:show="show">
    <div class="mm-dialog mm-panel">
      <div class="mm-dialog-title">标签</div>
      <div class="mm-tag-list">
        <span v-for="(tag, index) in tags" :key="index" class="mm-tag-chip">
          {{ tag }}
          <button type="button" class="mm-tag-del" @click="remove(index)">✕</button>
        </span>
        <span v-if="!tags.length" class="mm-dialog-hint">还没有标签</span>
      </div>
      <input v-model="draft" class="mm-dialog-input" placeholder="输入后按回车添加" @keydown.enter.prevent="add" />
      <div class="mm-dialog-actions">
        <button type="button" class="btn" @click="show = false">取消</button>
        <button type="button" class="btn primary" @click="confirm">确定</button>
      </div>
    </div>
  </NModal>
</template>
