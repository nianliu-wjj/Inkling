<script setup lang="ts">
import { NInputNumber } from 'naive-ui'
import FieldRow from './FieldRow.vue'

/** 数字字段。 */
const props = defineProps<{ label: string; modelValue: number; min?: number; max?: number; step?: number }>()
const emit = defineEmits<{ (e: 'update:modelValue', value: number): void }>()
</script>

<template>
  <FieldRow :label="label">
    <NInputNumber
      class="mm-number"
      size="small"
      :value="props.modelValue"
      :min="min"
      :max="max"
      :step="step ?? 1"
      @update:value="(v: number | null) => emit('update:modelValue', v ?? 0)"
    />
  </FieldRow>
</template>
