<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { logger } from '@/service/logger'
import { requireMindMap, useMindMap } from '../core/useMindMap'

/**
 * 演示模式（移植参考端 Demonstrate.vue）。
 * 进入 `mindMap.demonstrate.enter()`；演示中悬浮控制条：上一步 / 步数 / 下一步 / 跳转输入 / 退出。
 * 订阅库事件 `demonstrate_jump(index, total)` 更新步数，`exit_demonstrate` 复位状态。
 */
const ctx = useMindMap()
const { bus } = ctx

const isDemonstrating = ref(false)
const curStepIndex = ref(0)
const totalStep = ref(0)
const inputStep = ref('')

function enter(): void {
  isDemonstrating.value = true
  requireMindMap(ctx).demonstrate.enter()
  logger.info('mindmap', '进入演示模式')
}
function exit(): void {
  requireMindMap(ctx).demonstrate.exit()
}
function prev(): void {
  requireMindMap(ctx).demonstrate.prev()
}
function next(): void {
  requireMindMap(ctx).demonstrate.next()
}
/** 跳转到输入的步数（1 基）。 */
function onJumpInput(): void {
  const num = Number(inputStep.value)
  if (Number.isNaN(num)) {
    inputStep.value = ''
    return
  }
  if (num >= 1 && num <= totalStep.value) requireMindMap(ctx).demonstrate.jump(num - 1)
}

function onDemonstrateJump(index: number, total: number): void {
  curStepIndex.value = index
  totalStep.value = total
}
function onExitDemonstrate(): void {
  isDemonstrating.value = false
  curStepIndex.value = 0
  totalStep.value = 0
  logger.info('mindmap', '退出演示模式')
}

const offs: Array<() => void> = []
onMounted(() => {
  offs.push(
    bus.on('demonstrate_jump', (index, total) => onDemonstrateJump(index as number, total as number)),
    bus.on('exit_demonstrate', onExitDemonstrate),
  )
})
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <button type="button" class="mm-nav-btn iconfont iconyanshibofang" title="演示" @click="enter" />

  <!-- 演示中：退出按钮（右上）+ 步进控制条（右下），fixed 覆盖全屏演示画布 -->
  <Teleport to="body">
    <template v-if="isDemonstrating">
      <button type="button" class="mm-demo-exit" title="退出演示" @click="exit" @mousedown.stop @mouseup.stop>✕</button>
      <div class="mm-demo-step" @mousedown.stop @mouseup.stop>
        <button type="button" class="mm-demo-jump" :disabled="curStepIndex <= 0" title="上一步" @click="prev">‹</button>
        <span class="mm-demo-count">{{ curStepIndex + 1 }} / {{ totalStep }}</span>
        <button
          type="button"
          class="mm-demo-jump"
          :disabled="curStepIndex >= totalStep - 1"
          title="下一步"
          @click="next"
        >
          ›
        </button>
        <input
          v-model="inputStep"
          type="text"
          class="mm-demo-input"
          title="输入步数回车跳转"
          @keyup.enter.stop="onJumpInput"
          @keydown.stop
        />
      </div>
    </template>
  </Teleport>
</template>
