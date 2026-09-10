<script setup lang="ts">
/**
 * 原型内联 SVG 图标（docs/app.js 的 ICON_CLOSE / ICON_PIN / ICON_EDIT / ICON_PASTE / ICON_LINK）。
 * 11×11，描边取 currentColor，随按钮颜色变化；装饰性图标，可读名由外层按钮的 title 提供。
 */
export type IconName = 'close' | 'pin' | 'edit' | 'paste' | 'link'

defineProps<{ name: IconName }>()
</script>

<template>
  <svg v-if="name === 'close'" width="11" height="11" viewBox="0 0 12 12" fill="none" aria-hidden="true">
    <path d="M1 1l10 10M11 1L1 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
  </svg>
  <svg v-else-if="name === 'pin'" width="11" height="11" viewBox="0 0 24 24" fill="none" aria-hidden="true">
    <path d="M9 4h6l1 7 3 3v2H5v-2l3-3 1-7z" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" />
    <path d="M12 16v5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
  </svg>
  <svg v-else-if="name === 'edit'" width="11" height="11" viewBox="0 0 24 24" fill="none" aria-hidden="true">
    <path d="M4 20h4l11-11-4-4L4 16v4z" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" />
    <path d="M13 7l4 4" stroke="currentColor" stroke-width="1.6" />
  </svg>
  <svg v-else-if="name === 'paste'" width="11" height="11" viewBox="0 0 24 24" fill="none" aria-hidden="true">
    <path
      d="M16 4h2a2 2 0 012 2v14a2 2 0 01-2 2H6a2 2 0 01-2-2V6a2 2 0 012-2h2"
      stroke="currentColor"
      stroke-width="1.6"
      stroke-linecap="round"
    />
    <rect x="8" y="2" width="8" height="4" rx="1" stroke="currentColor" stroke-width="1.6" />
  </svg>
  <svg v-else width="11" height="11" viewBox="0 0 24 24" fill="none" aria-hidden="true">
    <path d="M14 3h7v7" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
    <path d="M21 3l-9 9" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
    <path
      d="M19 14v5a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2h5"
      stroke="currentColor"
      stroke-width="1.6"
      stroke-linecap="round"
    />
  </svg>
</template>
