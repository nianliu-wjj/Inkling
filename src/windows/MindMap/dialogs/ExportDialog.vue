<script setup lang="ts">
import { NModal } from 'naive-ui'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { downTypeList } from '../constants/lists'
import { requireMindMap, useMindMap } from '../core/useMindMap'
import { saveDataUrl } from '../core/exportFile'

/**
 * 导出对话框（移植参考端 Export.vue）。
 *
 * 选格式 + 选项，调 `mindMap.export(type, false, name, ...rest)` 拿 dataURL，
 * 再经系统保存框写盘。各格式的 rest 参数与参考端 confirm() 完全一致。
 */
const ctx = useMindMap()
const { bus, ui } = ctx
const { toast } = useToast()

const show = ref(false)
const exportType = ref('smm')
const fileName = ref('思维导图')
const withConfig = ref(true)
const isTransparent = ref(false)
const isFitBg = ref(true)
const paddingX = ref(10)
const paddingY = ref(10)
const footerText = ref('')
const exporting = ref(false)

const current = computed(() => downTypeList.find((t) => t.type === exportType.value))
/** md/xmind/txt/json 无图形选项。 */
const noImageOptions = computed(() => ['md', 'xmind', 'txt', 'json'].includes(exportType.value))
const showFitBg = computed(() => ['png', 'pdf'].includes(exportType.value) && !isTransparent.value)

function open(): void {
  show.value = true
}

async function confirm(): Promise<void> {
  const mindMap = requireMindMap(ctx)
  const type = exportType.value
  const name = fileName.value.trim() || '思维导图'
  ui.extraTextOnExport = footerText.value
  mindMap.updateConfig({ exportPaddingX: Number(paddingX.value), exportPaddingY: Number(paddingY.value) })
  exporting.value = true
  try {
    let dataUrl: string
    let ext = type
    if (type === 'svg') {
      dataUrl = await mindMap.export('svg', false, name, `* { margin: 0; padding: 0; box-sizing: border-box; }`)
    } else if (type === 'smm' || type === 'json') {
      dataUrl = await mindMap.export(type, false, name, withConfig.value)
    } else if (type === 'png') {
      dataUrl = await mindMap.export('png', false, name, isTransparent.value, null, isFitBg.value)
      ext = 'png'
    } else if (type === 'pdf') {
      dataUrl = await mindMap.export('pdf', false, name, isTransparent.value, isFitBg.value)
    } else {
      dataUrl = await mindMap.export(type, false, name)
    }
    const path = await saveDataUrl(dataUrl, name, ext)
    if (path) toast(`已导出到 ${path}`, 4000)
    show.value = false
  } catch (error) {
    logger.error('mindmap', '导出失败', error)
    toast('导出失败')
  } finally {
    exporting.value = false
  }
}

const offs: Array<() => void> = []
onMounted(() => offs.push(bus.on('showExport', open)))
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <NModal v-model:show="show" :mask-closable="!exporting">
    <div class="mm-dialog mm-panel">
      <div class="mm-dialog-title">导出</div>
      <div class="mm-export-formats">
        <button
          v-for="item in downTypeList"
          :key="item.type"
          type="button"
          class="mm-export-format"
          :class="{ active: exportType === item.type }"
          @click="exportType = item.type"
        >
          {{ item.name }}
        </button>
      </div>
      <p v-if="current" class="mm-dialog-hint">{{ current.desc }}</p>

      <label class="mm-field">
        <span class="mm-field-label">文件名</span>
        <input v-model="fileName" class="mm-dialog-input" />
      </label>
      <label v-if="exportType === 'smm' || exportType === 'json'" class="mm-field">
        <span class="mm-field-label">包含主题 / 结构等配置</span>
        <input v-model="withConfig" type="checkbox" />
      </label>
      <template v-if="!noImageOptions">
        <label v-if="['png', 'pdf'].includes(exportType)" class="mm-field">
          <span class="mm-field-label">背景透明</span>
          <input v-model="isTransparent" type="checkbox" />
        </label>
        <label v-if="showFitBg" class="mm-field">
          <span class="mm-field-label">显示完整背景图</span>
          <input v-model="isFitBg" type="checkbox" />
        </label>
        <label class="mm-field">
          <span class="mm-field-label">水平内边距</span>
          <input v-model.number="paddingX" type="number" class="mm-dialog-num" />
        </label>
        <label class="mm-field">
          <span class="mm-field-label">垂直内边距</span>
          <input v-model.number="paddingY" type="number" class="mm-dialog-num" />
        </label>
        <label class="mm-field">
          <span class="mm-field-label">底部附加文字</span>
          <input v-model="footerText" class="mm-dialog-input" placeholder="可留空" />
        </label>
      </template>

      <div class="mm-dialog-actions">
        <button type="button" class="btn" @click="show = false">取消</button>
        <button type="button" class="btn primary" :disabled="exporting" @click="confirm">
          {{ exporting ? '导出中…' : '导出' }}
        </button>
      </div>
    </div>
  </NModal>
</template>
