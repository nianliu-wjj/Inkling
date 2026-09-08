<script setup lang="ts">
/**
 * 表单行：左标签 + 右控件，侧栏里通用。控件放默认插槽。
 */
defineProps<{ label: string }>()
</script>

<template>
  <label class="mm-field">
    <span class="mm-field-label">{{ label }}</span>
    <span class="mm-field-control"><slot /></span>
  </label>
</template>
