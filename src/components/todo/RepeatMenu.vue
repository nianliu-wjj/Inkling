<script setup lang="ts">
import { nextTick, ref } from 'vue'
import { useAnchoredMenu } from '@/composables/useAnchoredMenu'
import { enter } from '@/motion'

/**
 * 重复提醒下拉菜单（原型 #repeatMenu / showRepeatMenu，spec §4.5）。
 *
 * 三项：🚫 不重复 / 🔁 每天重复 / 🔁 每周重复；当前规则带 .active；锚在 🔁 徽章下方 +6、左缘对齐、
 * 右缘不出视口（useAnchoredMenu 负责定位 / 翻转 / 点外关闭 / Esc / 键盘导航 / 焦点归还）；
 * 入场 y −6 → 0 淡入（--dur-base）。选中后 emit pick，由调用方写库并 toast。
 */
const emit = defineEmits<{ (e: 'pick', rule: string | null): void }>()

/** 顺序与原型一致；value '' 表示不重复（提交时转 null）。 */
const OPTIONS: readonly { value: '' | 'daily' | 'weekly'; label: string }[] = [
  { value: '', label: '🚫 不重复' },
  { value: 'daily', label: '🔁 每天重复' },
  { value: 'weekly', label: '🔁 每周重复' },
]

const { visible, activeIndex, menuRef, style, open: openMenu, close, onKeydown } = useAnchoredMenu(() => OPTIONS.length)

/** 当前规则：打开时传入，渲染 .active。 */
const current = ref<string>('')

function choose(index: number): void {
  const option = OPTIONS[index]
  if (!option) return
  close()
  emit('pick', option.value === '' ? null : option.value)
}

/** 由父组件调用：以 🔁 徽章为锚点打开，光标预置在当前规则上。 */
async function open(anchor: HTMLElement, rule: string | null): Promise<void> {
  current.value = rule ?? ''
  const index = OPTIONS.findIndex((option) => option.value === current.value)
  await openMenu(anchor, index < 0 ? 0 : index)
  // 定位完成后再播入场（原型 y −6 → 0，.18s）；菜单带 backdrop-filter，只在瞬时动效里加 transform。
  await nextTick()
  if (menuRef.value) void enter(menuRef.value, { axis: 'y', distance: -6, duration: 'base' })
}

/** 浮层统一收口（原型 switchMode 会一并隐藏 #repeatMenu / #prioMenu）：菜单开着才关并返回 true。 */
function dismiss(): boolean {
  if (!visible.value) return false
  close()
  return true
}

defineExpose({ open, close, dismiss })
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      ref="menuRef"
      class="repeat-menu"
      role="listbox"
      tabindex="-1"
      aria-label="选择重复提醒"
      :style="style"
      @keydown="onKeydown($event, choose)"
    >
      <div
        v-for="(option, index) in OPTIONS"
        :key="option.value"
        class="repeat-opt"
        :class="{ active: option.value === current }"
        :data-repeat="option.value"
        role="option"
        :aria-selected="option.value === current"
        @click="choose(index)"
        @mouseenter="activeIndex = index"
      >
        {{ option.label }}
      </div>
    </div>
  </Teleport>
</template>
