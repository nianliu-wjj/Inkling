<script setup lang="ts">
import xmind from 'simple-mind-map/src/parse/xmind.js'
import markdown from 'simple-mind-map/src/parse/markdown.js'
import { NModal, NTabPane, NTabs } from 'naive-ui'
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { useMindMap } from '../core/useMindMap'
import { useConfirm } from '../core/confirm'

/**
 * 导入对话框（移植参考端 Import.vue）。
 *
 * 支持 .smm / .json / .xmind / .md 文件，以及直接粘贴 Markdown 文本。
 * 解析成功后 `bus.emit('setData', data)`（MindMapApp 负责 setFullData/setData + 保存）。
 * xmind 多画布时弹选择。
 */
const { bus } = useMindMap()
const { toast } = useToast()
const confirm = useConfirm()

const show = ref(false)
const tab = ref<'file' | 'md'>('file')
const mdText = ref('')
const fileInput = ref<HTMLInputElement | null>(null)

function open(): void {
  show.value = true
  mdText.value = ''
}

function pickFile(): void {
  fileInput.value?.click()
}

async function onFile(event: Event): Promise<void> {
  const file = (event.target as HTMLInputElement).files?.[0]
  if (!file) return
  try {
    if (/\.(smm|json)$/i.test(file.name)) await handleSmm(file)
    else if (/\.xmind$/i.test(file.name)) await handleXmind(file)
    else if (/\.md$/i.test(file.name)) await handleMd(file)
    else toast('请选择 .smm / .json / .xmind / .md 文件')
  } catch (error) {
    logger.error('mindmap', '导入失败', error)
    toast('文件解析失败')
  } finally {
    ;(event.target as HTMLInputElement).value = ''
  }
}

function readText(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(String(reader.result))
    reader.onerror = () => reject(reader.error)
    reader.readAsText(file)
  })
}

async function handleSmm(file: File): Promise<void> {
  const data = JSON.parse(await readText(file))
  if (typeof data !== 'object') throw new Error('文件内容有误')
  finish(data)
}

async function handleXmind(file: File): Promise<void> {
  // 多画布：库回调传入画布列表，让用户选一个。
  const data = await xmind.parseXmindFile(file, (canvasList: unknown) => {
    const list = canvasList as Array<{ name?: string }>
    if (list.length <= 1) return Promise.resolve(list[0])
    // 简化：用原生 confirm 逐个问，选第一个「是」的；多数 xmind 单画布，够用。
    return (async () => {
      for (const canvas of list) {
        if (await confirm(`导入画布「${canvas.name ?? '未命名'}」？`, '选择画布')) return canvas
      }
      return list[0]
    })()
  })
  finish(data)
}

async function handleMd(file: File): Promise<void> {
  finish(markdown.transformMarkdownTo(await readText(file)))
}

function importMdText(): void {
  const text = mdText.value.trim()
  if (!text) {
    toast('内容不能为空')
    return
  }
  try {
    finish(markdown.transformMarkdownTo(text))
  } catch (error) {
    logger.error('mindmap', 'Markdown 解析失败', error)
    toast('文件解析失败')
  }
}

function finish(data: unknown): void {
  bus.emit('setData', data)
  toast('导入成功')
  show.value = false
}

const offs: Array<() => void> = []
onMounted(() => {
  offs.push(bus.on('showImport', open))
  offs.push(bus.on('importFile', (file) => void onFile({ target: { files: [file], value: '' } } as unknown as Event)))
})
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <NModal v-model:show="show">
    <div class="mm-dialog mm-panel">
      <div class="mm-dialog-title">导入</div>
      <NTabs v-model:value="tab" type="segment" size="small">
        <NTabPane name="file" tab="选择文件">
          <p class="mm-dialog-hint">支持 .smm / .json / .xmind / .md</p>
          <button type="button" class="btn" @click="pickFile">选取文件…</button>
          <input ref="fileInput" type="file" accept=".smm,.json,.xmind,.md" hidden @change="onFile" />
        </NTabPane>
        <NTabPane name="md" tab="粘贴 Markdown">
          <textarea
            v-model="mdText"
            class="mm-dialog-textarea"
            placeholder="粘贴 Markdown 格式内容（用标题与无序列表表达层级）"
          />
          <div class="mm-dialog-actions">
            <button type="button" class="btn" @click="show = false">取消</button>
            <button type="button" class="btn primary" @click="importMdText">导入</button>
          </div>
        </NTabPane>
      </NTabs>
    </div>
  </NModal>
</template>
