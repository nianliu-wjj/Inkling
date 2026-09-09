<script setup lang="ts">
import { useMindMap } from '../core/useMindMap'

/**
 * 鼠标行为切换（移植参考端 MouseAction.vue）：在「左键框选 / 右键拖动画布」与
 * 「左键拖动画布 / 右键框选」之间切换。只写 localConfig，由 MindMapApp 的 watch
 * 统一同步到库（updateConfig），避免双写。
 */
const { localConfig } = useMindMap()

function toggle(): void {
  localConfig.useLeftKeySelectionRightKeyDrag = !localConfig.useLeftKeySelectionRightKeyDrag
}
</script>

<template>
  <button
    type="button"
    class="mm-nav-btn iconfont"
    :class="localConfig.useLeftKeySelectionRightKeyDrag ? 'iconmouseR' : 'iconmouseL'"
    :title="
      localConfig.useLeftKeySelectionRightKeyDrag ? '当前：左键框选 / 右键拖动画布' : '当前：左键拖动画布 / 右键框选'
    "
    @click="toggle"
  />
</template>
