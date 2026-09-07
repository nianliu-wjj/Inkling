<script setup lang="ts">
import { computed } from 'vue'
import type { IslandItem } from '@/island-plugins'
import type { Todo } from '@/typings/domain'
import { formatStamp } from '@/utils/datetime'
import { isOverdue } from '@/utils/todo'

/**
 * 灵动岛悬停时展示的待办详情：标题全文、完成时间（含日期）、优先级、标签、备注前 120 字、逾期标记。
 */
const props = defineProps<{ item: IslandItem }>()

const todo = computed(() => props.item.payload as Todo)
const priorityLabel = computed(() => ({ high: '高', medium: '中', low: '低' })[todo.value.priority] ?? '')
const remark = computed(() =>
  todo.value.remark.length > 120 ? `${todo.value.remark.slice(0, 120)}…` : todo.value.remark,
)
</script>

<template>
  <div class="island-detail">
    <div class="island-detail-title">
      <i class="island-dot" :style="{ background: props.item.dot }" />
      <span>{{ todo.content }}</span>
      <span v-if="isOverdue(todo)" class="island-overdue">逾期</span>
    </div>
    <div class="island-detail-row">
      <span>⏰ {{ formatStamp(todo.due_at) }}</span>
      <span>优先级 {{ priorityLabel }}</span>
      <span v-if="todo.tags.length">🏷 {{ todo.tags.join(' · ') }}</span>
    </div>
    <div v-if="remark" class="island-detail-remark">{{ remark }}</div>
  </div>
</template>
