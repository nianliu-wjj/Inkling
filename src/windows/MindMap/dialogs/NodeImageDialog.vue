<script setup lang="ts">
import { NModal } from 'naive-ui'
import { onBeforeUnmount, onMounted, ref } from 'vue'
import type { MindMapNode } from 'simple-mind-map'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { useMindMap } from '../core/useMindMap'

/**
 * 节点图片对话框（移植参考端 NodeImage.vue）。
 *
 * 选图片 → FileReader 转 dataURL + `new Image()` 取宽高 → 确认写 `node.setImage`。
 * 打开时回显当前节点已有的图片标题。
 */
const { bus, ui } = useMindMap()
const { toast } = useToast()

const show = ref(false)
const node = ref<MindMapNode | null>(null)
const url = ref('')
const title = ref('')
const width = ref(0)
const height = ref(0)
const fileInput = ref<HTMLInputElement | null>(null)

function open(): void {
  node.value = ui.activeNodes[0] ?? null
  if (!node.value) {
    toast('请先选择一个节点')
    return
  }
  const img = node.value.getData('image')
  url.value = img || ''
  title.value = node.value.getData('imageTitle') || ''
  const size = node.value.getData('imageSize')
  width.value = size?.width ?? 0
  height.value = size?.height ?? 0
  show.value = true
}

function pick(): void {
  fileInput.value?.click()
}

function onFile(event: Event): void {
  const file = (event.target as HTMLInputElement).files?.[0]
  if (!file) return
  const reader = new FileReader()
  reader.onload = () => {
    const dataUrl = String(reader.result)
    const image = new Image()
    image.onload = () => {
      url.value = dataUrl
      width.value = image.width
      height.value = image.height
    }
    image.onerror = () => toast('图片加载失败')
    image.src = dataUrl
  }
  reader.readAsDataURL(file)
  ;(event.target as HTMLInputElement).value = ''
}

function confirm(): void {
  if (!node.value) return
  try {
    node.value.setImage(
      url.value
        ? { url: url.value, title: title.value, width: width.value, height: height.value }
        : { url: '', title: '', width: 0, height: 0 },
    )
    show.value = false
  } catch (error) {
    logger.error('mindmap', '设置节点图片失败', error)
    toast('设置失败')
  }
}

function removeImage(): void {
  url.value = ''
  confirm()
}

const offs: Array<() => void> = []
onMounted(() => offs.push(bus.on('showNodeImage', open)))
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <NModal v-model:show="show">
    <div class="mm-dialog mm-panel">
      <div class="mm-dialog-title">节点图片</div>
      <div v-if="url" class="mm-image-preview"><img :src="url" alt="预览" /></div>
      <button type="button" class="btn" @click="pick">选择图片…</button>
      <input ref="fileInput" type="file" accept="image/*" hidden @change="onFile" />
      <label class="mm-field">
        <span class="mm-field-label">图片标题</span>
        <input v-model="title" class="mm-dialog-input" placeholder="可留空" />
      </label>
      <div class="mm-dialog-actions">
        <button v-if="url" type="button" class="btn" @click="removeImage">移除图片</button>
        <button type="button" class="btn" @click="show = false">取消</button>
        <button type="button" class="btn primary" @click="confirm">确定</button>
      </div>
    </div>
  </NModal>
</template>
