<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'

/**
 * 弹窗外壳：遮罩 + 毛玻璃面板 + 标题栏 + Esc 关闭。
 *
 * 样式沿用原型：遮罩与面板的尺寸/层级挂在 ID 选择器上
 * （#tagManagerOverlay / #todoEditorOverlay / #clipEditorOverlay），
 * 因此调用方需通过 overlayId / modalId 传入对应 ID。
 *
 * 整体 Teleport 到 body：遮罩是 position: fixed，若留在调用方子树内，
 * 会被带 backdrop-filter 的祖先（如面板的 .glass）当作包含块，
 * 只能覆盖祖先自身的盒子，弹窗随之被裁切。挂到 body 后始终以窗口为基准。
 * 面板窗口会另行把 .modal-overlay / .modal-shell 覆写为整页编辑器（见 window-fit.css）。
 */
const props = defineProps<{
  /** 遮罩元素 ID，决定层级与遮罩样式。 */
  overlayId: string
  /** 面板元素 ID，决定弹窗宽度与内边距。 */
  modalId: string
  title: string
}>()

const emit = defineEmits<{ (e: 'close'): void }>()

/** Esc 关闭。用 capture 保证优先于内部控件的键盘处理。 */
function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    event.stopPropagation()
    emit('close')
  }
}

onMounted(() => document.addEventListener('keydown', onKeydown, true))
onBeforeUnmount(() => document.removeEventListener('keydown', onKeydown, true))
</script>

<template>
  <Teleport to="body">
    <!-- 点击遮罩空白处关闭；点击面板内部不冒泡到遮罩。 -->
    <div :id="props.overlayId" class="modal-overlay" @click.self="emit('close')">
      <div :id="props.modalId" class="glass modal-shell" role="dialog" aria-modal="true" :aria-label="title">
        <div class="clip-editor-header">
          <span class="clip-editor-title">{{ title }}</span>
          <button type="button" class="icon-btn" title="关闭（Esc）" @click="emit('close')">
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
              <path d="M1 1l10 10M11 1L1 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
            </svg>
          </button>
        </div>

        <div class="modal-body">
          <slot />
        </div>

        <div v-if="$slots.footer" class="clip-editor-footer">
          <slot name="footer" />
        </div>
      </div>
    </div>
  </Teleport>
</template>
