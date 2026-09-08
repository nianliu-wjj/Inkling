<script setup lang="ts">
import { NColorPicker } from 'naive-ui'
import { colorList } from '../constants/lists'
import FieldRow from './FieldRow.vue'

/**
 * 颜色字段：36 色快捷色板 + naive-ui 取色器（支持透明与更多颜色）。
 * v-model 绑定 CSS 颜色字符串。
 */
const props = defineProps<{ label: string; modelValue: string }>()
const emit = defineEmits<{ (e: 'update:modelValue', value: string): void }>()

function pick(color: string): void {
  emit('update:modelValue', color)
}
</script>

<template>
  <FieldRow :label="label">
    <div class="mm-color">
      <button
        v-for="color in colorList"
        :key="color"
        type="button"
        class="mm-color-dot"
        :class="{ active: color === props.modelValue, transparent: color === 'transparent' }"
        :style="color === 'transparent' ? undefined : { background: color }"
        :title="color"
        @click="pick(color)"
      />
      <NColorPicker
        class="mm-color-more"
        :value="props.modelValue"
        :modes="['hex', 'rgb']"
        size="small"
        @update:value="pick"
      />
    </div>
  </FieldRow>
</template>
