<script setup lang="ts">
/**
 * 删除二次确认浮层。
 *
 * 需求 2.2：绝对定位于卡片**上方**，不占满卡片、不推挤布局。
 * 样式与入场动画完全复用原型的 .card-confirm / confirmIn，
 * 因此宿主卡片必须是 position: relative。
 */
withDefaults(defineProps<{ text?: string }>(), { text: '⚠️ 确认删除？' })

defineEmits<{ (e: 'confirm'): void; (e: 'cancel'): void }>()
</script>

<template>
  <div class="card-confirm" role="alertdialog" :aria-label="text">
    <span class="card-confirm-text">{{ text }}</span>
    <button type="button" class="btn tiny danger" @click.stop="$emit('confirm')">删除</button>
    <button type="button" class="btn tiny ghost" @click.stop="$emit('cancel')">取消</button>
  </div>
</template>
