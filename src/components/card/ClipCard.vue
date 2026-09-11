<script setup lang="ts">
import { computed } from 'vue'
import ConfirmPopover from '@/components/base/ConfirmPopover.vue'
import Icon from '@/components/base/Icon.vue'
import IconBtn from '@/components/base/IconBtn.vue'
import ClipTypeBadge from '@/components/clip/ClipTypeBadge.vue'
import type { ClipboardEntry } from '@/typings/domain'
import { formatStamp } from '@/utils/datetime'

/**
 * 面板剪贴板卡片（原型 renderClips 的 .clip-item）。
 *
 * - 右上角 `.card-close`（悬浮显示，二次确认走卡片上方浮层）；
 * - 头部 `.clip-head`：时间（置顶带 📌 前缀）→ 来源应用 `.clip-from`（spec D20，无值不渲染）→ 类型徽章；
 * - 正文最多两行，超出省略；
 * - 右下 `.clip-ops`：粘贴 / 打开链接(仅 link) / 编辑(仅文本类) / 收藏置顶，全部内联 SVG 图标；
 *   「粘贴」是直接粘贴到光标处（面板收起、焦点归还后模拟 Ctrl/Cmd+V）；
 * - 双击 = 粘贴到光标处并置顶；置顶条目金色高亮并优先排序（排序由调用方负责）。
 */
const props = withDefaults(defineProps<{ entry: ClipboardEntry; confirming?: boolean }>(), { confirming: false })

const emit = defineEmits<{
  (e: 'paste'): void
  (e: 'edit'): void
  (e: 'pin'): void
  (e: 'open-link'): void
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

    <button type="button" class="card-close" title="删除该条目" @click="emit('ask-delete')">
      <Icon name="close" />
    </button>

    <div class="clip-head">
      <span class="clip-time"
        ><template v-if="props.entry.pinned"><span class="ix">📌</span> </template>{{ stamp }}</span
      >
      <span v-if="props.entry.source_app" class="clip-from" title="来源应用"
        ><span class="ix">🌐</span> {{ props.entry.source_app }}</span
      >
      <ClipTypeBadge :content-type="props.entry.content_type" />
    </div>

    <div class="clip-text">{{ props.entry.preview || props.entry.content }}</div>

    <div class="clip-ops">
      <IconBtn title="粘贴（直接粘贴到光标处）" @click="emit('paste')"><Icon name="paste" /></IconBtn>
      <IconBtn v-if="isLink" variant="clip-open" title="用默认浏览器打开该链接" @click="emit('open-link')">
        <Icon name="link" />
      </IconBtn>
      <IconBtn v-if="editable" title="编辑内容" @click="emit('edit')"><Icon name="edit" /></IconBtn>
      <IconBtn
        :variant="props.entry.pinned ? 'active-pin' : ''"
        :title="props.entry.pinned ? '取消收藏' : '收藏置顶'"
        @click="emit('pin')"
      >
        <Icon name="pin" />
      </IconBtn>
    </div>
  </li>
</template>
