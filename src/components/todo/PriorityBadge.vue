<script setup lang="ts">
import type { Priority } from '@/typings/domain'

/**
 * 优先级徽章。
 *
 * 需求 2.2：三级优先级以不同颜色徽章标识（高=红 / 中=黄 / 低=绿）；
 * 点击打开锚定式选择菜单。**已完成项仅展示非交互徽章**。
 */
const props = withDefaults(
  defineProps<{
    priority: Priority
    /** 已完成项不可修改，渲染为非交互徽章。 */
    readonly?: boolean
  }>(),
  { readonly: false },
)

const emit = defineEmits<{ (e: 'open', anchor: HTMLElement): void }>()

const LABELS: Record<Priority, string> = { high: '高', medium: '中', low: '低' }

function open(event: MouseEvent | KeyboardEvent): void {
  if (props.readonly) return
  emit('open', event.currentTarget as HTMLElement)
}
</script>

<template>
  <span
    class="prio-badge"
    :class="props.priority"
    :tabindex="props.readonly ? -1 : 0"
    :role="props.readonly ? undefined : 'button'"
    :aria-haspopup="props.readonly ? undefined : 'listbox'"
    :aria-label="`优先级：${LABELS[props.priority]}`"
    :style="props.readonly ? { cursor: 'default' } : undefined"
    :title="props.readonly ? '已完成事项不可修改优先级' : '点击调整优先级'"
    @click.stop="open"
    @keydown.enter.prevent.stop="open"
    @keydown.space.prevent.stop="open"
  >
    {{ LABELS[props.priority] }}
  </span>
</template>
