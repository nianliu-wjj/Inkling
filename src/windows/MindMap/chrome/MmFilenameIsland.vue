<script setup lang="ts">
import { nextTick, ref } from 'vue'
import { logger } from '@/service/logger'
import { MAP_NAME_LIMIT, normalizeMapName } from '@/utils/mindmapName'

/**
 * 文件名岛（原型 `#mmFilenameIsland`，`docs/index.html:333-346`）。
 *
 * 形态照原型：常态是文本 + ✏️ 按钮，编辑态是输入框 + 💾 按钮；Enter / 失焦 / 💾 提交，
 * Esc 取消。**不渲染后缀徽章与后缀下拉**（spec D44）：我们的导图存进笔记
 * （`notes.content` 即导图名、`mindmap_data` 是导图数据），没有「保存格式」概念，
 * 原型那 4 个后缀实测也只是字符串、落库无差异。
 */
const props = defineProps<{ name: string }>()
const emit = defineEmits<{ rename: [name: string] }>()

const editing = ref(false)
const draft = ref('')
const input = ref<HTMLInputElement | null>(null)

async function startEdit(): Promise<void> {
  draft.value = props.name
  editing.value = true
  await nextTick()
  input.value?.focus()
  input.value?.select()
  logger.debug('mindmap', '文件名岛进入编辑态')
}

function commit(): void {
  if (!editing.value) return
  const name = normalizeMapName(draft.value)
  editing.value = false
  if (name === props.name) return
  logger.info('mindmap', `导图改名为「${name}」`)
  emit('rename', name)
}

function cancel(): void {
  editing.value = false
  logger.debug('mindmap', '文件名岛放弃改名')
}
</script>

<template>
  <div class="mm-filename-island" @keydown.esc.stop="cancel">
    <template v-if="editing">
      <input
        ref="input"
        v-model="draft"
        class="mm-filename-input"
        :maxlength="MAP_NAME_LIMIT"
        spellcheck="false"
        autocomplete="off"
        @keydown.enter.prevent="commit"
        @blur="commit"
      />
      <button type="button" class="mm-filename-btn" title="保存名称" @mousedown.prevent @click="commit">💾</button>
    </template>
    <template v-else>
      <span class="mm-filename-text" title="点击编辑导图名" @click="startEdit">{{ name }}</span>
      <button type="button" class="mm-filename-btn" title="编辑导图名" @click="startEdit">✏️</button>
    </template>
  </div>
</template>
