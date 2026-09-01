<script setup lang="ts">
import { ref } from 'vue'
import ModalShell from '@/components/base/ModalShell.vue'
import { logger } from '@/service/logger'

/**
 * 剪贴板内容编辑浮框。
 *
 * 需求 2.2：编辑与置顶是两个独立功能——编辑弹框回显原文，保存后替换原内容、
 * 时间更新为最后修改时间，且**不影响置顶状态**；仅文本类条目可编辑。
 * 支持 ⌃/⌘+Enter 快捷保存。
 */
const props = defineProps<{ content: string }>()

const emit = defineEmits<{
  (e: 'save', content: string): void
  (e: 'close'): void
}>()

const draft = ref(props.content)

function save(): void {
  logger.info('clip-editor', `保存剪贴板内容，长度 ${draft.value.length}`)
  emit('save', draft.value)
}
</script>

<template>
  <ModalShell
    overlay-id="clipEditorOverlay"
    modal-id="clipEditorModal"
    title="✏️ 编辑剪贴板内容"
    @close="emit('close')"
  >
    <textarea
      id="clipEditorTextarea"
      v-model="draft"
      placeholder="编辑内容…"
      spellcheck="false"
      @keydown.ctrl.enter.prevent="save"
      @keydown.meta.enter.prevent="save"
    />

    <template #footer>
      <span class="clip-editor-hint"> 保存后替换原内容，时间更新为最后修改时间（⌃/⌘+Enter 快捷保存） </span>
      <div class="clip-editor-actions">
        <button type="button" class="btn ghost" @click="emit('close')">取消</button>
        <button type="button" class="btn primary" @click="save">保存修改</button>
      </div>
    </template>
  </ModalShell>
</template>
