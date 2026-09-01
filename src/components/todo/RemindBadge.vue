<script setup lang="ts">
import { computed } from 'vue'
import { formatRemindLabel } from '@/utils/datetime'

/**
 * 提醒徽章。
 *
 * 需求 2.2：
 * - 已设提醒 → 常显「⏰ 日期 时间」（与完成同日仅显时间），点击进入提醒模式编辑；
 * - 未设置 → 淡色 ⏰ 占位，悬浮提示默认提醒计划（完成前 30 分 / 前 5 分 / 到点各一次）。
 */
const props = withDefaults(
  defineProps<{
    remindAt: string | null
    dueAt: string
    readonly?: boolean
  }>(),
  { readonly: false },
)

const emit = defineEmits<{ (e: 'edit'): void }>()

const label = computed(() => (props.remindAt ? formatRemindLabel(props.remindAt, props.dueAt) : ''))

const title = computed(() =>
  props.remindAt
    ? '点击修改提醒时间'
    : '未设置提醒时间，将在完成时间前 30 分钟、前 5 分钟与到点时各提醒一次；点击可设置',
)
</script>

<template>
  <span
    class="remind-badge"
    :class="{ 'no-remind': !props.remindAt }"
    :title="title"
    @click.stop="!props.readonly && emit('edit')"
  >
    ⏰<template v-if="label"> {{ label }}</template>
  </span>
</template>
