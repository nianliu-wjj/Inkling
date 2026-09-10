<script setup lang="ts">
import { computed } from 'vue'
import ConfirmPopover from '@/components/base/ConfirmPopover.vue'
import Icon from '@/components/base/Icon.vue'
import IconBtn from '@/components/base/IconBtn.vue'
import ClipTypeBadge from '@/components/clip/ClipTypeBadge.vue'
import type { ClipboardEntry } from '@/typings/domain'
import { formatStamp } from '@/utils/datetime'

/**
 * 主窗口粘贴板归档卡片（原型 renderArchive 的 .archive-item.clip-arch）：
 * 正文最多两行（CSS 截断）、类型徽章、时间、右下操作组（粘贴 / 置顶 / 外链仅 link / 编辑仅文本类）。
 * 面板里的紧凑卡片是另一个组件（ClipCard），阶段三对齐。
 * 「粘贴」是直接粘贴到光标处（面板收起、焦点归还后模拟 Ctrl/Cmd+V），不只是写回剪贴板。
 */
const props = withDefaults(defineProps<{ entry: ClipboardEntry; confirming?: boolean }>(), { confirming: false })

const emit = defineEmits<{
  (e: 'paste'): void
  (e: 'pin'): void
  (e: 'edit'): void
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
  <div class="archive-item clip-arch" :class="{ pinned: props.entry.pinned }">
    <ConfirmPopover
      v-if="props.confirming"
      text="⚠️ 确认删除该剪贴板条目？"
      @confirm="emit('confirm-delete')"
      @cancel="emit('cancel-delete')"
    />

    <button type="button" class="card-close" title="删除该条目" @click="emit('ask-delete')">
      <Icon name="close" />
    </button>

    <div class="a-text">{{ props.entry.preview || props.entry.content }}</div>

    <div class="a-meta">
      <ClipTypeBadge :content-type="props.entry.content_type" />
      <span
        ><template v-if="props.entry.pinned"><span class="ix">📌</span> </template>{{ stamp }}</span
      >

      <span class="a-ops">
        <IconBtn title="粘贴到鼠标光标处" @click="emit('paste')"><Icon name="paste" /></IconBtn>
        <IconBtn
          :variant="props.entry.pinned ? 'active-pin' : ''"
          :title="props.entry.pinned ? '取消置顶' : '置顶'"
          @click="emit('pin')"
        >
          <Icon name="pin" />
        </IconBtn>
        <IconBtn v-if="isLink" variant="clip-open" title="用默认浏览器打开该链接" @click="emit('open-link')">
          <Icon name="link" />
        </IconBtn>
        <IconBtn v-if="editable" title="编辑内容（弹框回显修改）" @click="emit('edit')">
          <Icon name="edit" />
        </IconBtn>
      </span>
    </div>
  </div>
</template>
