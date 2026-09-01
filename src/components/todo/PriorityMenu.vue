<script setup lang="ts">
import { ref } from 'vue'
import { useAnchoredMenu } from '@/composables/useAnchoredMenu'
import type { Priority } from '@/typings/domain'

/**
 * 优先级锚定选择菜单。
 *
 * 需求 v1.2 变更 #6：三项**始终完整展示**，当前值带勾选与选中态，
 * 不再用卡片本身充当菜单项；颜色圆点 + 文字双通道（颜色不作唯一辨识信息）；
 * 默认向下展开，空间不足向上翻转；支持键盘与 Esc/外部点击关闭。
 *
 * 定位、翻转、键盘导航、焦点归还统一由 useAnchoredMenu 负责。
 */
const emit = defineEmits<{ (e: 'select', priority: Priority): void }>()

/** 顺序固定为 高 → 中 → 低，与徽章语义一致。 */
const OPTIONS: readonly { value: Priority; label: string }[] = [
  { value: 'high', label: '高' },
  { value: 'medium', label: '中' },
  { value: 'low', label: '低' },
]

const { visible, activeIndex, menuRef, style, open: openMenu, close, onKeydown } = useAnchoredMenu(() => OPTIONS.length)

/** 当前值：打开时传入，用于渲染勾选态。 */
const current = ref<Priority>('medium')

function choose(index: number): void {
  const option = OPTIONS[index]
  if (!option) return
  close()
  emit('select', option.value)
}

/** 由父组件调用：以徽章为锚点打开菜单，并把光标预置在当前值上。 */
async function open(anchor: HTMLElement, priority: Priority): Promise<void> {
  current.value = priority
  const index = OPTIONS.findIndex((o) => o.value === priority)
  await openMenu(anchor, index < 0 ? 0 : index)
}

defineExpose({ open, close })
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      ref="menuRef"
      class="prio-group"
      role="listbox"
      tabindex="-1"
      aria-label="选择优先级"
      :style="style"
      @keydown="onKeydown($event, choose)"
    >
      <div
        v-for="(option, index) in OPTIONS"
        :key="option.value"
        class="prio-opt"
        :class="[option.value, { active: activeIndex === index }]"
        role="option"
        :aria-selected="option.value === current"
        @click="choose(index)"
        @mouseenter="activeIndex = index"
      >
        <i class="prio-dot" />
        <span>{{ option.label }}</span>
        <span class="prio-check" aria-hidden="true">✓</span>
      </div>
    </div>
  </Teleport>
</template>
