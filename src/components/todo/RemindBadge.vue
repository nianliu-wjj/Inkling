<script setup lang="ts">
import { computed } from 'vue'
import { remindOffsetLabel } from '@/constants/reminder'

/**
 * 提醒徽章。
 *
 * 展示用户选择的提醒偏移与启用的渠道，点击进入提醒编辑。
 * 偏移为 null（不提醒）时显示淡色 ⏰ 占位。
 */
const props = withDefaults(
  defineProps<{
    offsetMinutes: number | null
    desktop: boolean
    email: boolean
    readonly?: boolean
  }>(),
  { readonly: false },
)

const emit = defineEmits<{ (e: 'edit'): void }>()

const label = computed(() => (props.offsetMinutes === null ? '' : remindOffsetLabel(props.offsetMinutes)))

/** 渠道图标：弹窗与邮件各占一个，都没勾时不显示。 */
const channels = computed(() => {
  const marks: string[] = []
  if (props.desktop) marks.push('🔔')
  if (props.email) marks.push('✉️')
  return marks.join('')
})

const title = computed(() =>
  props.offsetMinutes === null
    ? '未设置提醒；点击可设置'
    : `将在完成时间${remindOffsetLabel(props.offsetMinutes)}与到点各提醒一次；点击修改`,
)
</script>

<template>
  <span
    class="remind-badge"
    :class="{ 'no-remind': props.offsetMinutes === null }"
    :title="title"
    @click.stop="!props.readonly && emit('edit')"
  >
    ⏰<template v-if="label"> {{ label }}{{ channels ? ` ${channels}` : '' }}</template>
  </span>
</template>
