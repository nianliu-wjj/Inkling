<script setup lang="ts">
import { onBeforeUnmount, reactive, watch } from 'vue'
import { logger } from '@/service/logger'
import { useConfirm } from '../core/confirm'
import { requireMindMap, useMindMap } from '../core/useMindMap'
import ColorField from '../widgets/ColorField.vue'
import FieldRow from '../widgets/FieldRow.vue'
import SelectField from '../widgets/SelectField.vue'
import SwitchField from '../widgets/SwitchField.vue'
import NumberField from '../widgets/NumberField.vue'
import SidebarShell from './SidebarShell.vue'

/**
 * 设置侧栏（移植参考端 Setting.vue）。
 *
 * 库实例配置绑定到 reactive `mapConfig`（MindMapApp 里 watch → updateConfig + 落盘）；
 * 编辑器行为偏好绑定到 `localConfig`（MindMapApp watch 完成插件挂卸等）。
 * 富文本开关切换会清空历史，先弹确认。水印直接调 `mindMap.watermark.updateWatermark`。
 */
const ctx = useMindMap()
const { mapConfig, localConfig } = ctx
const confirm = useConfirm()

const mousewheelActions = [
  { name: '缩放视图', value: 'zoom' },
  { name: '上下移动视图', value: 'move' },
]
const zoomReverse = [
  { name: '向前缩小 / 向后放大', value: true },
  { name: '向前放大 / 向后缩小', value: false },
]
const createBehaviors = [
  { name: '激活新节点并进入编辑', value: 'default' },
  { name: '不激活新节点', value: 'notActive' },
  { name: '只激活不进入编辑', value: 'activeOnly' },
]

/** 富文本开关：清空历史的破坏性操作，先确认。 */
async function toggleRichText(value: boolean): Promise<void> {
  const mindMap = requireMindMap(ctx)
  mindMap.renderer.textEdit?.hideEditTextBox?.()
  const ok = await confirm('该操作会清空所有历史修改记录并修改思维导图数据，是否继续？', '切换富文本模式')
  if (!ok) return
  localConfig.openNodeRichText = value
}

// —— 水印（参考端 initWatermark / updateWatermarkConfig / watermarkShowChange）——

interface WatermarkConfig {
  show: boolean
  onlyExport: boolean
  belowNode: boolean
  text: string
  lineSpacing: number
  textSpacing: number
  angle: number
  textStyle: { color: string; opacity: number; fontSize: number }
}

/** 水印配置本地副本：从库实例读，更新时防抖写回并同步到 mapConfig。 */
const watermarkConfig = reactive<WatermarkConfig>({
  show: false,
  onlyExport: false,
  belowNode: false,
  text: '',
  lineSpacing: 100,
  textSpacing: 100,
  angle: 30,
  textStyle: { color: '', opacity: 0, fontSize: 1 },
})

/** 打开侧栏 / 挂载时从库实例回显（show 以是否有水印文字为准，与参考端一致）。 */
function syncWatermark(): void {
  const config = (requireMindMap(ctx).getConfig('watermarkConfig') ?? {}) as Record<string, unknown>
  watermarkConfig.show = !!config.text
  watermarkConfig.onlyExport = !!config.onlyExport
  watermarkConfig.belowNode = !!config.belowNode
  watermarkConfig.text = (config.text as string) ?? ''
  watermarkConfig.lineSpacing = (config.lineSpacing as number) ?? 100
  watermarkConfig.textSpacing = (config.textSpacing as number) ?? 100
  watermarkConfig.angle = (config.angle as number) ?? 30
  watermarkConfig.textStyle = {
    color: (config.textStyle as { color?: string })?.color ?? '',
    opacity: (config.textStyle as { opacity?: number })?.opacity ?? 0,
    fontSize: (config.textStyle as { fontSize?: number })?.fontSize ?? 1,
  }
}

let updateWatermarkTimer: ReturnType<typeof setTimeout> | null = null

/** 防抖 300ms 写回水印；随后把库实例的最新配置同步进 mapConfig 落盘（参考端 storeConfig）。 */
function updateWatermark(): void {
  if (updateWatermarkTimer) clearTimeout(updateWatermarkTimer)
  updateWatermarkTimer = setTimeout(() => {
    updateWatermarkTimer = null
    const mindMap = requireMindMap(ctx)
    const { show, ...config } = watermarkConfig
    mindMap.watermark?.updateWatermark({ ...config })
    mapConfig.watermarkConfig = mindMap.getConfig('watermarkConfig')
    logger.info('mindmap', `更新水印 show=${show} text=${watermarkConfig.text || '(空)'}`)
  }, 300)
}

/** 切换水印开关：开启时空文案补默认值「水印文字」，关闭时清空文字（参考端 watermarkShowChange）。 */
function toggleWatermark(value: boolean): void {
  watermarkConfig.show = value
  watermarkConfig.text = value ? watermarkConfig.text || '水印文字' : ''
  updateWatermark()
}

watch(
  () => ctx.ui.activeSidebar,
  (name) => {
    if (name === 'setting') syncWatermark()
  },
)
syncWatermark()

onBeforeUnmount(() => {
  if (updateWatermarkTimer) clearTimeout(updateWatermarkTimer)
})
</script>

<template>
  <SidebarShell name="setting" title="设置">
    <div class="mm-setting-group">编辑</div>
    <SwitchField
      label="开启节点富文本编辑"
      :model-value="localConfig.openNodeRichText"
      @update:model-value="toggleRichText"
    />
    <SwitchField
      label="键盘输入时自动进入编辑"
      :model-value="!!mapConfig.enableAutoEnterTextEditWhenKeydown"
      @update:model-value="(v) => (mapConfig.enableAutoEnterTextEditWhenKeydown = v)"
    />
    <SwitchField
      label="文本编辑实时渲染"
      :model-value="!!mapConfig.openRealtimeRenderOnNodeTextEdit"
      @update:model-value="(v) => (mapConfig.openRealtimeRenderOnNodeTextEdit = v)"
    />
    <SelectField
      label="新建节点行为"
      :model-value="(mapConfig.createNewNodeBehavior as string) ?? 'default'"
      :options="createBehaviors"
      @update:model-value="(v) => (mapConfig.createNewNodeBehavior = v)"
    />

    <div class="mm-setting-group">画布</div>
    <SelectField
      label="鼠标滚轮行为"
      :model-value="(mapConfig.mousewheelAction as string) ?? 'zoom'"
      :options="mousewheelActions"
      @update:model-value="(v) => (mapConfig.mousewheelAction = v)"
    />
    <SelectField
      label="滚轮缩放方向"
      :model-value="mapConfig.mousewheelZoomActionReverse !== false"
      :options="zoomReverse"
      @update:model-value="(v) => (mapConfig.mousewheelZoomActionReverse = v)"
    />
    <SwitchField
      label="左键框选 / 右键拖动画布"
      :model-value="localConfig.useLeftKeySelectionRightKeyDrag"
      @update:model-value="(v) => (localConfig.useLeftKeySelectionRightKeyDrag = v)"
    />
    <SwitchField
      label="显示滚动条"
      :model-value="localConfig.isShowScrollbar"
      @update:model-value="(v) => (localConfig.isShowScrollbar = v)"
    />
    <SwitchField
      label="拖拽文件到窗口导入"
      :model-value="localConfig.enableDragImport"
      @update:model-value="(v) => (localConfig.enableDragImport = v)"
    />
    <SwitchField
      label="拖动画布动量效果"
      :model-value="!!mapConfig.isUseMomentum"
      @update:model-value="(v) => (mapConfig.isUseMomentum = v)"
    />

    <div class="mm-setting-group">外观与性能</div>
    <SwitchField
      label="手绘风格"
      :model-value="!!mapConfig.isUseHandDrawnLikeStyle"
      @update:model-value="(v) => (mapConfig.isUseHandDrawnLikeStyle = v)"
    />
    <SwitchField
      label="一直显示展开 / 收起按钮"
      :model-value="!!mapConfig.alwaysShowExpandBtn"
      @update:model-value="(v) => (mapConfig.alwaysShowExpandBtn = v)"
    />
    <SwitchField
      label="连线样式继承祖先"
      :model-value="!!mapConfig.enableInheritAncestorLineStyle"
      @update:model-value="(v) => (mapConfig.enableInheritAncestorLineStyle = v)"
    />
    <SwitchField
      label="性能模式（Beta）"
      :model-value="!!mapConfig.openPerformance"
      @update:model-value="(v) => (mapConfig.openPerformance = v)"
    />
    <NumberField
      label="图片与文本间隔"
      :model-value="(mapConfig.imgTextMargin as number) ?? 5"
      :min="0"
      :max="50"
      @update:model-value="(v) => (mapConfig.imgTextMargin = v)"
    />
    <NumberField
      label="节点内容间隔"
      :model-value="(mapConfig.textContentMargin as number) ?? 2"
      :min="0"
      :max="50"
      @update:model-value="(v) => (mapConfig.textContentMargin = v)"
    />

    <div class="mm-setting-group">水印</div>
    <SwitchField label="是否显示水印" :model-value="watermarkConfig.show" @update:model-value="toggleWatermark" />
    <template v-if="watermarkConfig.show">
      <SwitchField
        label="仅导出时显示"
        :model-value="watermarkConfig.onlyExport"
        @update:model-value="(v) => ((watermarkConfig.onlyExport = v), updateWatermark())"
      />
      <SwitchField
        label="显示在节点下方"
        :model-value="watermarkConfig.belowNode"
        @update:model-value="(v) => ((watermarkConfig.belowNode = v), updateWatermark())"
      />
      <FieldRow label="水印文字">
        <input v-model="watermarkConfig.text" class="mm-dialog-input" type="text" @change="updateWatermark" />
      </FieldRow>
      <ColorField
        label="文字颜色"
        :model-value="watermarkConfig.textStyle.color || '#999999'"
        @update:model-value="(v) => ((watermarkConfig.textStyle.color = v), updateWatermark())"
      />
      <FieldRow label="文字透明度">
        <input
          :value="watermarkConfig.textStyle.opacity"
          class="mm-setting-range"
          type="range"
          min="0"
          max="1"
          step="0.1"
          @input="
            ((watermarkConfig.textStyle.opacity = Number(($event.target as HTMLInputElement).value)), updateWatermark())
          "
        />
      </FieldRow>
      <NumberField
        label="文字字号"
        :model-value="watermarkConfig.textStyle.fontSize"
        :min="0"
        :max="50"
        @update:model-value="(v) => ((watermarkConfig.textStyle.fontSize = v), updateWatermark())"
      />
      <NumberField
        label="旋转角度"
        :model-value="watermarkConfig.angle"
        :min="0"
        :max="90"
        :step="10"
        @update:model-value="(v) => ((watermarkConfig.angle = v), updateWatermark())"
      />
      <NumberField
        label="水印行间距"
        :model-value="watermarkConfig.lineSpacing"
        :step="10"
        @update:model-value="(v) => ((watermarkConfig.lineSpacing = v), updateWatermark())"
      />
      <NumberField
        label="水印文字间距"
        :model-value="watermarkConfig.textSpacing"
        :step="10"
        @update:model-value="(v) => ((watermarkConfig.textSpacing = v), updateWatermark())"
      />
    </template>
  </SidebarShell>
</template>
