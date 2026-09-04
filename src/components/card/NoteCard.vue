<script setup lang="ts">
import { computed } from 'vue'
import ConfirmPopover from '@/components/base/ConfirmPopover.vue'
import IconBtn from '@/components/base/IconBtn.vue'
import TagList from '@/components/tag/TagList.vue'
import type { Note } from '@/typings/domain'
import { formatStamp } from '@/utils/datetime'
import { renderMarkdown } from '@/utils/format'

/**
 * 笔记卡片。
 *
 * 需求 2.2：
 * - 正文按 Markdown 渲染，不展示源码标记；
 * - 元数据行：时间与标签 chips **同一行**，标签紧跟在时间之后，最多 3 个、
 *   超出以「+N」聚合；无标签时显示置灰占位（不可删除，点击弹标签管理）；
 * - ✕ 在右上角、功能按钮在底部右侧，均仅悬浮卡片时显示（由 CSS 负责）。
 */
const props = withDefaults(
  defineProps<{
    note: Note
    confirming?: boolean
  }>(),
  { confirming: false },
)

const emit = defineEmits<{
  (e: 'edit'): void
  (e: 'pin'): void
  (e: 'open-tags'): void
  (e: 'ask-delete'): void
  (e: 'confirm-delete'): void
  (e: 'cancel-delete'): void
}>()

/** 归档时刻优先，草稿回落到创建时刻。 */
const stamp = computed(() => formatStamp(props.note.archived_at ?? props.note.created_at))

/** 是否思维导图笔记：列表里要能一眼分辨两种类型。 */
const isMindmap = computed(() => props.note.editor_mode === 'mindmap')
const mindmapLabel = computed(() => (isMindmap.value ? '🧠 思维导图笔记' : ''))
const html = computed(() =>
  props.note.editor_mode === 'mindmap'
    ? `<p class="note-mindmap-summary">${mindmapLabel.value}</p>`
    : renderMarkdown(props.note.content),
)
</script>

<template>
  <div class="archive-item" :class="{ pinned: props.note.pinned, mindmap: isMindmap }">
    <ConfirmPopover
      v-if="props.confirming"
      text="⚠️ 确认删除该笔记？"
      @confirm="emit('confirm-delete')"
      @cancel="emit('cancel-delete')"
    />

    <IconBtn variant="card-close" title="删除" @click="emit('ask-delete')">✕</IconBtn>

    <!-- 正文由 markdown-it 渲染（html:false，不信任原始 HTML） -->
    <div class="a-text" v-html="html" />

    <div class="a-meta">
      <!-- 类型徽章：思维导图与文本笔记在列表里混排，需要固定位置的类型标识 -->
      <span class="note-kind" :class="isMindmap ? 'kind-mindmap' : 'kind-text'">
        {{ isMindmap ? '🧠 导图' : '📝 笔记' }}
      </span>
      <span>{{ stamp }}</span>
      <TagList :tags="props.note.tags" :max="3" @open="emit('open-tags')" />

      <div class="a-ops">
        <IconBtn title="编辑" @click="emit('edit')">✏️</IconBtn>
        <IconBtn
          :variant="props.note.pinned ? 'active-pin' : ''"
          :title="props.note.pinned ? '取消置顶' : '置顶到桌面'"
          @click="emit('pin')"
          >📌</IconBtn
        >
      </div>
    </div>
  </div>
</template>
