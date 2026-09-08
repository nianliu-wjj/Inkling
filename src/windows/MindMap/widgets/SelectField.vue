<script setup lang="ts">
import { NSelect } from 'naive-ui'
import { computed } from 'vue'
import FieldRow from './FieldRow.vue'

/**
 * 下拉字段。options 为 { name, value }[]，值类型任意（字符串/数字/布尔）。
 */
const props = defineProps<{
  label: string
  modelValue: string | number | boolean
  options: readonly { name: string; value: string | number | boolean }[]
}>()
const emit = defineEmits<{ (e: 'update:modelValue', value: string | number | boolean): void }>()

const naiveOptions = computed(() => props.options.map((o) => ({ label: o.name, value: o.value as never })))
</script>

<template>
  <FieldRow :label="label">
    <NSelect
      class="mm-select"
      size="small"
      :value="props.modelValue as never"
      :options="naiveOptions"
      @update:value="(v: string | number | boolean) => emit('update:modelValue', v)"
    />
  </FieldRow>
</template>
