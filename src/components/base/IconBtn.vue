<script setup lang="ts">
/**
 * 图标按钮。
 *
 * 对应原型 .icon-btn（22×22 圆角、hover 提亮）。
 * 卡片上的显隐由父容器的 CSS :hover 规则控制（如 .archive-item:hover .a-ops），
 * 组件自身不持有 hover 状态。
 */
withDefaults(
  defineProps<{
    /** 原生 title，用作悬浮提示。 */
    title?: string
    /** 追加的类名，例如 clip-open / active-pin。 */
    variant?: string
    disabled?: boolean
  }>(),
  { title: '', variant: '', disabled: false },
)

defineEmits<{ (e: 'click', event: MouseEvent): void }>()
</script>

<template>
  <button
    type="button"
    class="icon-btn"
    :class="variant"
    :title="title"
    :disabled="disabled"
    @click="$emit('click', $event)"
  >
    <slot />
  </button>
</template>
