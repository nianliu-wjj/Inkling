<script setup lang="ts">
import { NModal } from 'naive-ui'
import { onBeforeUnmount, onMounted, ref } from 'vue'
import type { MindMapNode } from 'simple-mind-map'
import { getTextFromHtml } from 'simple-mind-map/src/utils/index.js'
import { useToast } from '@/composables/useToast'
import { requireMindMap, useMindMap } from '../core/useMindMap'

/**
 * 节点链接对话框（移植参考端「链接到指定节点」流程）。
 *
 * 与「超链接」（外部 URL，见 NodeHyperlinkDialog）区分：此处把节点链接到画布内另一个节点，
 * 存为 `#<目标 uid>` 形式的 hyperlink（点击时由 MindMapApp 的 hyperlinkJump 处理跳转）。
 * 事件名用 `showNodeLinkToNode` 避免与超链接对话框的 `showNodeLink` 冲突。
 *
 * 流程：打开 → 进入「选择目标节点」态 → 监听一次 node_click → 不能链接自己（提示）→
 * 确认后 `node.setHyperlink('#'+targetUid, 目标文本)`；可选「同时添加反向链接」；可删除本节点链接。
 */
const ctx = useMindMap()
const { bus } = ctx
const { toast } = useToast()

const show = ref(false)
const sourceNode = ref<MindMapNode | null>(null)
const picking = ref(false)
const addReverse = ref(false)
const targetText = ref('')
let offNodeClick: (() => void) | null = null

function open(node: MindMapNode): void {
  sourceNode.value = node
  picking.value = false
  addReverse.value = false
  targetText.value = ''
  show.value = true
}

/** 进入选择目标节点态：监听一次 node_click。 */
function startPick(): void {
  if (!sourceNode.value) return
  picking.value = true
  toast('请在画布中点击要链接到的目标节点')
  offNodeClick?.()
  offNodeClick = bus.on('node_click', (target) => onPickTarget(target as MindMapNode))
}

function onPickTarget(target: MindMapNode): void {
  offNodeClick?.()
  offNodeClick = null
  picking.value = false
  const source = sourceNode.value
  if (!source || !target) return
  if (target === source) {
    toast('不能链接到自己')
    return
  }
  const targetUid = target.getData('uid') as string
  const text = pureText(target)
  targetText.value = text
  source.setHyperlink(`#${targetUid}`, text)
  if (addReverse.value) {
    const sourceUid = source.getData('uid') as string
    target.setHyperlink(`#${sourceUid}`, pureText(source))
  }
  toast('已建立节点链接')
  show.value = false
}

/** 取节点纯文本（富文本剥标签）。 */
function pureText(node: MindMapNode): string {
  const raw = String(node.getData('text') ?? '')
  return node.getData('richText') ? getTextFromHtml(raw) : raw
}

/** 删除本节点的链接。 */
function removeLink(): void {
  sourceNode.value?.setHyperlink('', '')
  toast('已删除节点链接')
  show.value = false
}

function onShow(node: MindMapNode): void {
  open(node)
}

const offs: Array<() => void> = []
onMounted(() => {
  offs.push(bus.on('showNodeLinkToNode', (node) => onShow(node as MindMapNode)))
})
onBeforeUnmount(() => {
  offs.forEach((off) => off())
  offNodeClick?.()
})
</script>

<template>
  <NModal v-model:show="show">
    <div class="mm-dialog mm-panel">
      <div class="mm-dialog-title">链接到指定节点</div>
      <p class="mm-dialog-hint">
        点击「选择目标节点」后，在画布中点选要链接到的节点；已选：{{ targetText || '（未选择）' }}
      </p>
      <label class="mm-linknode-reverse">
        <input v-model="addReverse" type="checkbox" />
        同时为目标节点添加返回本节点的反向链接
      </label>
      <div class="mm-dialog-actions">
        <button type="button" class="btn" @click="removeLink">删除本节点链接</button>
        <span style="flex: 1"></span>
        <button type="button" class="btn" @click="show = false">取消</button>
        <button type="button" class="btn primary" :disabled="picking" @click="startPick">
          {{ picking ? '请在画布点选…' : '选择目标节点' }}
        </button>
      </div>
    </div>
  </NModal>
</template>
