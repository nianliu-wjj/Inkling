<script setup lang="ts">
/**
 * 标签 chip。
 *
 * 需求 2.2：
 * - ✕ 删除按钮仅在鼠标悬浮于该标签上时显示（由原型 CSS .tag-chip:hover .tag-del 控制）；
 * - 进入抖动确认态时 ✕ 强制常显并以 0.7s/次抖动变红（.tag-chip.shaking）；
 * - 悬浮到 ✕ 上时抖动暂停，由 CSS :has(.tag-del:hover) 负责，此处无需处理。
 */
const props = withDefaults(
  defineProps<{
    label: string
    /** 是否处于抖动二次确认态。 */
    shaking?: boolean
    /** 是否允许删除；只读场景（如已完成待办）隐藏 ✕。 */
    deletable?: boolean
  }>(),
  { shaking: false, deletable: true },
)

const emit = defineEmits<{ (e: 'click'): void; (e: 'remove'): void }>()
</script>

<template>
  <span class="tag-chip" :class="{ shaking: props.shaking }" :title="props.label" @click="emit('click')">
    <span class="tag-name">{{ props.label }}</span>
    <!-- 阻止冒泡，避免点 ✕ 时同时触发 chip 的编辑行为。 -->
    <i v-if="props.deletable" class="tag-del" title="删除标签" @click.stop="emit('remove')">✕</i>
  </span>
</template>
