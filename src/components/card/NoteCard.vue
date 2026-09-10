<script setup lang="ts">
import { computed } from 'vue'
import ConfirmPopover from '@/components/base/ConfirmPopover.vue'
import Icon from '@/components/base/Icon.vue'
import IconBtn from '@/components/base/IconBtn.vue'
import TagList from '@/components/tag/TagList.vue'
import type { Note } from '@/typings/domain'
import { formatStamp } from '@/utils/datetime'
import { renderMarkdown, renderMarkdownInline } from '@/utils/format'
import { mindmapRootText } from '@/utils/search'

/**
 * 笔记卡片（原型 renderArchive 的 .archive-item）。
 *
 * - 右上角 ✕（悬浮显示）；正文按 Markdown 渲染；思维导图笔记正文前置「🧠 思维导图」徽章，
 *   正文为根节点文字；
 * - 元数据行：📌? 时间 · 标签 chips（最多 3 个、超出 +N；✕ 悬浮显示，两次点击抖动确认删除）
 *   · 右侧操作组（置顶 / 编辑，悬浮显示）；
 * - 抖动确认态（`shaking`）下卡片加 .shaking：标签全部展开、✕ 常显、显示提示。
 */
const props = withDefaults(
  defineProps<{
    note: Note
    confirming?: boolean
    /** 卡片处于标签删除的抖动确认态。 */
    shaking?: boolean
    /** 抖动确认中的标签名。 */
    shakingTag?: string | null
  }>(),
  { confirming: false, shaking: false, shakingTag: null },
)

const emit = defineEmits<{
  (e: 'edit'): void
  (e: 'pin'): void
  (e: 'open-tags'): void
  (e: 'remove-tag', tag: string): void
  (e: 'ask-delete'): void
  (e: 'confirm-delete'): void
  (e: 'cancel-delete'): void
}>()

/** 归档时刻优先，草稿回落到创建时刻。 */
const stamp = computed(() => formatStamp(props.note.archived_at ?? props.note.created_at))

/** 是否思维导图笔记：列表里要能一眼分辨两种类型。 */
const isMindmap = computed(() => props.note.editor_mode === 'mindmap')

/** 正文：导图笔记显示根节点文字（原型 saveMindmap.rootText），文本笔记按 Markdown 渲染。 */
const html = computed(() =>
  isMindmap.value ? renderMarkdownInline(mindmapRootText(props.note.mindmap_data)) : renderMarkdown(props.note.content),
)
</script>

<template>
  <div class="archive-item" :class="{ pinned: props.note.pinned, shaking: props.shaking }">
    <ConfirmPopover
      v-if="props.confirming"
      text="⚠️ 确认删除该笔记？"
      @confirm="emit('confirm-delete')"
      @cancel="emit('cancel-delete')"
    />

    <button type="button" class="card-close" title="删除该笔记" @click="emit('ask-delete')">
      <Icon name="close" />
    </button>

    <!-- 正文由 markdown-it 渲染（html:false，不信任原始 HTML） -->
    <div class="a-text">
      <span v-if="isMindmap" class="clip-type mindmap"><span class="ix">🧠</span> 思维导图</span>
      <span v-html="html" />
    </div>

    <div class="a-meta">
      <span
        ><template v-if="props.note.pinned"><span class="ix">📌</span> </template>{{ stamp }}</span
      >
      <TagList
        :tags="props.note.tags"
        :max="3"
        deletable
        :shaking="props.shaking"
        :shaking-tag="props.shakingTag"
        @open="emit('open-tags')"
        @remove="emit('remove-tag', $event)"
      />

      <span class="a-ops">
        <IconBtn
          :variant="props.note.pinned ? 'active-pin' : ''"
          :title="props.note.pinned ? '取消置顶' : '置顶'"
          @click="emit('pin')"
        >
          <Icon name="pin" />
        </IconBtn>
        <IconBtn title="编辑" @click="emit('edit')"><Icon name="edit" /></IconBtn>
      </span>
    </div>
  </div>
</template>
