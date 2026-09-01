<script setup lang="ts">
import { computed, ref } from 'vue'
import TagChip from './TagChip.vue'

/**
 * 标签列表。
 *
 * 需求 2.2：
 * - 最多展示 `max` 个，超出以「+N」聚合，点击展开全部；
 * - 无标签时显示置灰「无标签」占位，占位**不可删除**，点击弹标签管理窗。
 *
 * 单个标签的字数上限由数据层保证（笔记 5 字 / 待办 10 字），
 * 视觉上再由 .tag-name 的 max-width + 省略号兜底。
 */
const props = withDefaults(
  defineProps<{
    tags: readonly string[]
    /** 折叠前展示的标签数量上限。 */
    max?: number
    /** 是否显示 ✕ 删除按钮。 */
    deletable?: boolean
    /** 当前处于抖动确认态的标签名。 */
    shakingTag?: string | null
  }>(),
  { max: 3, deletable: false, shakingTag: null },
)

const emit = defineEmits<{
  /** 点击标签本身（进入编辑）或点击「无标签」占位。 */
  (e: 'open'): void
  (e: 'remove', tag: string): void
}>()

/** 展开后不再折叠，直到组件重新挂载。 */
const expanded = ref(false)

const visibleTags = computed(() => (expanded.value ? props.tags : props.tags.slice(0, props.max)))

const hiddenCount = computed(() => Math.max(0, props.tags.length - props.max))
</script>

<template>
  <span class="a-tags">
    <template v-if="props.tags.length">
      <TagChip
        v-for="tag in visibleTags"
        :key="tag"
        :label="tag"
        :deletable="props.deletable"
        :shaking="props.shakingTag === tag"
        @click="emit('open')"
        @remove="emit('remove', tag)"
      />
      <span
        v-if="!expanded && hiddenCount > 0"
        class="tag-more"
        :title="`展开其余 ${hiddenCount} 个标签`"
        @click.stop="expanded = true"
      >
        +{{ hiddenCount }}
      </span>
    </template>

    <!-- 无标签占位：不可删除，点击进入标签管理。 -->
    <span v-else class="tag-empty" title="点击管理标签" @click="emit('open')">无标签</span>
  </span>
</template>
