<script setup lang="ts">
import { NSwitch } from 'naive-ui'
import FieldRow from './FieldRow.vue'

/** 开关字段。 */
const props = defineProps<{ label: string; modelValue: boolean }>()
const emit = defineEmits<{ (e: 'update:modelValue', value: boolean): void }>()
</script>

<template>
  <FieldRow :label="label">
    <NSwitch :value="props.modelValue" size="small" @update:value="(v: boolean) => emit('update:modelValue', v)" />
  </FieldRow>
</template>
