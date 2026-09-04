<script setup lang="ts">
import { useToast } from '@/composables/useToast'

/**
 * 轻提示宿主。
 *
 * 每个窗口在根组件放置一个即可，状态由模块级的 useToast 共享。
 * 样式复用原型 #toast。
 */
const { message, visible } = useToast()
</script>

<template>
  <Transition name="toast-fade">
    <div v-if="visible" id="toast" role="status" aria-live="polite">{{ message }}</div>
  </Transition>
</template>

<style scoped>
/* 淡入淡出：原型用 GSAP 驱动，此处用 Vue 过渡实现等效观感。
   时长与曲线走 tokens.css 的动效令牌，reduced-motion 下自动归零。 */
.toast-fade-enter-active,
.toast-fade-leave-active {
  transition:
    opacity var(--dur-base) var(--ease-out),
    transform var(--dur-base) var(--ease-out);
}
.toast-fade-enter-from,
.toast-fade-leave-to {
  opacity: 0;
  transform: translate(-50%, 6px);
}
</style>
