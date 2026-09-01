<script setup lang="ts">
import { computed, ref } from 'vue'
import ModalShell from '@/components/base/ModalShell.vue'
import { useShakeConfirm } from '@/composables/useShakeConfirm'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'

/**
 * 标签管理弹窗。
 *
 * 需求 2.2：流式布局；支持新增（回车、去重、字数上限）、修改（点击文字就地
 * 编辑）、删除（✕ 抖动二次确认）。管理页的 ✕ 常显，与卡片上的悬浮显示不同。
 */
const props = withDefaults(
  defineProps<{
    /** 当前标签，组件内部维持副本，保存时才回传。 */
    tags: readonly string[]
    /** 数量上限：笔记无硬上限取 99，待办为 3。 */
    maxCount?: number
    /** 单个标签字数上限：笔记 5 字，待办 10 字。 */
    maxLength?: number
    /** 副标题，说明当前管理的对象。 */
    subtitle?: string
  }>(),
  { maxCount: 99, maxLength: 5, subtitle: '当前笔记的标签' },
)

const emit = defineEmits<{
  (e: 'save', tags: string[]): void
  (e: 'close'): void
}>()

const { toast } = useToast()
const shake = useShakeConfirm()

/** 编辑副本：弹窗内的增删改先落在这里，关闭时统一回传。 */
const draft = ref<string[]>([...props.tags])
const input = ref('')

const canAdd = computed(() => draft.value.length < props.maxCount)

/** 归一化：去空白；空串与超长在调用处拦截。 */
function normalize(raw: string): string {
  return raw.trim()
}

function add(): void {
  const name = normalize(input.value)
  if (!name) return

  if (!canAdd.value) {
    toast(`最多只能添加 ${props.maxCount} 个标签`)
    return
  }
  if (name.length > props.maxLength) {
    toast(`标签最多 ${props.maxLength} 个字`)
    return
  }
  if (draft.value.includes(name)) {
    toast('该标签已存在')
    return
  }

  logger.info('tag-manager', `新增标签 ${name}`)
  draft.value.push(name)
  input.value = ''
}

/** 就地编辑：失焦时提交，校验失败则回滚为原值。 */
function rename(index: number, event: FocusEvent): void {
  const element = event.target as HTMLElement
  const next = normalize(element.textContent ?? '')
  const previous = draft.value[index]

  if (!next) {
    element.textContent = previous
    return
  }
  if (next.length > props.maxLength) {
    toast(`标签最多 ${props.maxLength} 个字`)
    element.textContent = previous
    return
  }
  if (next !== previous && draft.value.includes(next)) {
    toast('该标签已存在')
    element.textContent = previous
    return
  }

  logger.info('tag-manager', `重命名标签 ${previous} → ${next}`)
  draft.value[index] = next
}

/** ✕ 走抖动二次确认：第一次进入抖动态，第二次才真正删除。 */
function remove(tag: string): void {
  if (!shake.press(tag)) return
  draft.value = draft.value.filter((t) => t !== tag)
}

function save(): void {
  logger.info('tag-manager', `保存标签 [${draft.value.join(', ')}]`)
  emit('save', [...draft.value])
}
</script>

<template>
  <ModalShell
    overlay-id="tagManagerOverlay"
    modal-id="tagManagerModal"
    title="🏷️ 管理标签"
    @close="emit('close')"
  >
    <div class="tag-mgr-sub">{{ props.subtitle }}</div>

    <div class="tag-add-row">
      <input
        v-model="input"
        class="search-input"
        :placeholder="`输入标签名，回车添加（最多 ${props.maxLength} 字）`"
        :maxlength="props.maxLength"
        @keydown.enter.prevent="add"
      />
      <button type="button" class="btn primary" @click="add">添加</button>
    </div>

    <ul class="tag-mgr-list">
      <li
        v-for="(tag, index) in draft"
        :key="tag"
        class="tag-mgr-item"
        :class="{ shaking: shake.isArmed(tag) }"
      >
        <!-- contenteditable 实现就地编辑；回车提交走 blur。 -->
        <span
          class="tag-mgr-name"
          contenteditable="true"
          spellcheck="false"
          @blur="rename(index, $event)"
          @keydown.enter.prevent="($event.target as HTMLElement).blur()"
        >
          {{ tag }}
        </span>
        <i
          class="tag-del"
          :class="{ confirm: shake.isArmed(tag) }"
          :title="shake.isArmed(tag) ? '再次点击确认删除' : '删除标签'"
          @click="remove(tag)"
          >✕</i
        >
      </li>
      <li v-if="!draft.length" class="tag-mgr-empty">还没有标签，在上方输入框添加</li>
    </ul>

    <template #footer>
      <span class="clip-editor-hint">点击标签文字可直接修改 · ✕ 需点两次确认删除</span>
      <div class="clip-editor-actions">
        <button type="button" class="btn ghost" @click="emit('close')">取消</button>
        <button type="button" class="btn primary" @click="save">保存</button>
      </div>
    </template>
  </ModalShell>
</template>
