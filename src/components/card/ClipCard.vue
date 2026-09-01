<script setup lang="ts">
import { computed } from 'vue'
import ConfirmPopover from '@/components/base/ConfirmPopover.vue'
import IconBtn from '@/components/base/IconBtn.vue'
import ClipTypeBadge from '@/components/clip/ClipTypeBadge.vue'
import type { ClipboardEntry } from '@/typings/domain'
import { formatStamp } from '@/utils/datetime'

/**
 * 剪贴板卡片。
 *
 * 需求 2.2「粘贴板历史」的布局约定：
 * - 左上角：复制时间 / 最后修改时间；
 * - 右上角：✕ 删除（悬浮显示，二次确认走卡片上方浮层）；
 * - 右下角：操作组（粘贴 / 打开链接(仅 link) / 编辑(仅文本类) / 收藏置顶），悬浮显示；
 * - 内容最多两行，超出省略号截断（面板与归档一致）；
 * - 双击 = 粘贴并置顶；置顶条目金色高亮并优先排序（排序由调用方负责）。
 */
const props = withDefaults(
  defineProps<{
    entry: ClipboardEntry
    /** 归档页形态：额外展示类型徽章与「在浏览器打开」按钮。 */
    archive?: boolean
    confirming?: boolean
  }>(),
  { archive: false, confirming: false },
)

const emit = defineEmits<{
  (e: 'paste'): void
  (e: 'edit'): void
  (e: 'pin'): void
  (e: 'open-link'): void
  (e: 'export'): void
  (e: 'ask-delete'): void
  (e: 'confirm-delete'): void
  (e: 'cancel-delete'): void
}>()

/** 时间口径：内容被修改过则显示最后修改时间。 */
const stamp = computed(() => formatStamp(props.entry.modified_at || props.entry.copied_at))

/** 仅文本类条目可编辑（图片无正文可改）。 */
const editable = computed(() => props.entry.content_type !== 'image')
const isLink = computed(() => props.entry.content_type === 'link')
</script>

<template>
  <li class="clip-item" :class="{ pinned: props.entry.pinned }" :title="props.entry.preview" @dblclick="emit('paste')">
    <ConfirmPopover
      v-if="props.confirming"
      text="⚠️ 确认删除该剪贴板条目？"
      @confirm="emit('confirm-delete')"
      @cancel="emit('cancel-delete')"
    />

    <IconBtn variant="card-close" title="删除" @click="emit('ask-delete')">✕</IconBtn>

    <div class="clip-head">
      <span class="clip-time">{{ stamp }}</span>
      <ClipTypeBadge v-if="props.archive" :content-type="props.entry.content_type" />
    </div>

    <div class="clip-text">{{ props.entry.preview || props.entry.content }}</div>

    <div class="clip-ops">
      <IconBtn title="粘贴到剪贴板" @click="emit('paste')">📋</IconBtn>
      <IconBtn v-if="isLink" variant="clip-open" title="在默认浏览器中打开" @click="emit('open-link')">🔗</IconBtn>
      <IconBtn v-if="editable" title="编辑内容" @click="emit('edit')">✏️</IconBtn>
      <IconBtn
        :variant="props.entry.pinned ? 'active-pin' : ''"
        :title="props.entry.pinned ? '取消收藏置顶' : '收藏并置顶'"
        @click="emit('pin')"
        >★</IconBtn
      >
      <IconBtn v-if="props.archive" title="导出" @click="emit('export')">⤓</IconBtn>
    </div>
  </li>
</template>
