<script setup lang="ts">
import { NModal } from 'naive-ui'
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useToast } from '@/composables/useToast'
import { requireMindMap, useMindMap } from '../core/useMindMap'

/**
 * 源码编辑对话框（移植参考端源码编辑入口）。
 *
 * `ui.isSourceCodeEdit` 为真时打开，展示 `getData(true)` 的全量 JSON（含 root/layout/theme/view）；
 * 支持格式化、复制、完成（JSON.parse 校验后 `bus.emit('setData', data)` 写回，失败 Toast「JSON 格式有误」）。
 * 用样式化 textarea 承载（不引入需联网拉取的 @codemirror/lang-json）。
 */
const ctx = useMindMap()
const { bus, ui } = ctx
const { toast } = useToast()

const show = ref(false)
const code = ref('')

/** 打开时从实例读取全量数据的美化 JSON。 */
function load(): void {
  try {
    const data = requireMindMap(ctx).getData(true)
    code.value = JSON.stringify(data, null, 2)
  } catch {
    code.value = ''
  }
}

function format(): void {
  try {
    code.value = JSON.stringify(JSON.parse(code.value), null, 2)
  } catch {
    toast('JSON 格式有误')
  }
}

async function copy(): Promise<void> {
  try {
    await navigator.clipboard.writeText(code.value)
    toast('已复制到剪贴板')
  } catch {
    toast('复制失败')
  }
}

/** 完成：校验 JSON 后写回画布；失败提示且不关闭。 */
function done(): void {
  let data: unknown
  try {
    data = JSON.parse(code.value)
  } catch {
    toast('JSON 格式有误')
    return
  }
  bus.emit('setData', data)
  ui.isSourceCodeEdit = false
}

function close(): void {
  ui.isSourceCodeEdit = false
}

// 与 ui.isSourceCodeEdit 双向同步：打开时载入数据。
watch(
  () => ui.isSourceCodeEdit,
  (open) => {
    show.value = open
    if (open) load()
  },
)
watch(show, (v) => {
  if (!v) ui.isSourceCodeEdit = false
})

// Esc 关闭由 NModal 处理；这里额外确保初始状态一致。
onMounted(() => {
  show.value = ui.isSourceCodeEdit
  if (show.value) load()
})
onBeforeUnmount(() => {
  show.value = false
})
</script>

<template>
  <NModal v-model:show="show">
    <div class="mm-dialog mm-source mm-panel">
      <div class="mm-dialog-title">源码编辑</div>
      <textarea v-model="code" class="mm-dialog-textarea mm-source-area" spellcheck="false" @keydown.stop></textarea>
      <div class="mm-dialog-actions">
        <button type="button" class="btn" @click="format">格式化</button>
        <button type="button" class="btn" @click="copy">复制</button>
        <span class="mm-source-spacer" />
        <button type="button" class="btn" @click="close">取消</button>
        <button type="button" class="btn primary" @click="done">完成</button>
      </div>
    </div>
  </NModal>
</template>
