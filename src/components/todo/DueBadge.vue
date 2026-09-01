<script setup lang="ts">
import { computed } from 'vue'
import { formatDueLabel } from '@/utils/datetime'

/**
 * 完成时间徽章。
 *
 * 需求 2.2：以「📅」常显于卡片底部标签之后（父/子任务均有），
 * 格式「📅 今天 HH:mm」或「📅 M/D HH:mm」，逾期时红色；
 * 点击弹出「修改完成时间」聚焦弹窗（仅完成日期/时刻可编辑）。
 */
const props = withDefaults(
  defineProps<{
    dueAt: string
    overdue?: boolean
    /** 已完成事项不可修改完成时间。 */
    readonly?: boolean
  }>(),
  { overdue: false, readonly: false },
)

const emit = defineEmits<{ (e: 'edit'): void }>()

const label = computed(() => formatDueLabel(props.dueAt))
</script>

<template>
  <span
    class="due-badge"
    :class="{ overdue: props.overdue }"
    :style="props.readonly ? { cursor: 'default' } : undefined"
    :title="props.readonly ? '已完成事项不可修改完成时间' : '点击修改完成时间'"
    @click.stop="!props.readonly && emit('edit')"
  >
    📅 {{ label }}
  </span>
</template>
