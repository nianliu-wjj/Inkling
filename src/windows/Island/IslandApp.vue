<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch, type CSSProperties } from 'vue'
import { useSettings, useTodos } from '@/composables/useData'
import { applyCachedGlass, useGlass } from '@/composables/useGlass'
import { applyCachedTheme, useTheme } from '@/composables/useTheme'
import { resolveIslandPlugins, type IslandItem } from '@/island-plugins'
import { AppEvents, onAppEvent } from '@/service/events'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'

/**
 * 灵动岛根组件（原型 #dynamicIsland / renderIsland / diStep，spec 4B §4.1）。
 *
 * - DOM 按原型：`.di-ticker > .di-track > .di-item*`，条目末尾追加第一条副本做无缝回环；
 * - 逐条停留轮播：每条静止 `island_cycle_seconds` 秒 → `translateY(-i*h)` 0.45s ease → 滚到副本后 480ms
 *   无过渡复位（diStep）；仅一条或空态静止；展开 / 提醒 / 面板打开时不步进；
 * - 悬停 / 点击不用 DOM 鼠标事件，统一订阅后端推送的 island-hover / island-click（穿透模式下窗口收不到鼠标事件）；
 *   悬停穿透（D33）开关为真时忽略悬停事件（后端也不发，双保险）；
 * - 提醒岛内呈现：reminder-fired（payload = todo id）→ `.di-alert`「⏰ 提醒：内容」6s 或点击恢复；
 * - 面板展开淡出：panel-shown → `.di-fade`，panel-hidden / 下一次悬停事件 → 恢复；
 * - 右下角手柄：宽按 Δx×2 对称、高按 Δy，钳制 200–480 / 32–56，松手 `api.island.resize` 落库（穿透时不渲染手柄；
 *   展开态也渲染，否则任何悬停都会展开导致手柄不可达）；
 * - 手柄拖拽 / 提醒卡点击期间调 `api.island.setInteracting(true)`，后端暂停左键点击探测，避免被当成点击胶囊呼出面板。
 */
applyCachedTheme()
applyCachedGlass()
document.documentElement.dataset.window = 'island'

const { settings } = useSettings()
const { todos } = useTodos()
const { applyTheme } = useTheme()
const { applyGlass } = useGlass()

watch(
  () => settings.value.theme,
  (theme) => applyTheme(theme),
  { immediate: true },
)
watch(
  () => settings.value.glass_level,
  (level) => applyGlass(level),
  { immediate: true },
)

// ── 条目 ──

// 插件在 setup 内各调用一次 useItems（组合式函数，不能放进 computed 里反复调用）。
const pluginItems = resolveIslandPlugins(settings.value.island_plugins).map((plugin) => ({
  plugin,
  items: plugin.useItems(),
}))
const enabledIds = computed(() => resolveIslandPlugins(settings.value.island_plugins).map((p) => p.id))
/** 全部条目：按设置里的插件顺序拼接。 */
const items = computed<IslandItem[]>(() =>
  enabledIds.value.flatMap((id) => pluginItems.find((entry) => entry.plugin.id === id)?.items.value ?? []),
)
/** 空态文案（原型按播放范围二选一）。 */
const emptyText = computed(() => {
  if (settings.value.island_scope === 'all') return '没有未完成待办 🎉'
  return (
    pluginItems.find((entry) => enabledIds.value.includes(entry.plugin.id))?.plugin.emptyText ?? '今日待办已全部完成 🎉'
  )
})

/** 轨道上的一行（原型 .di-item 的渲染参数）。 */
interface IslandRow {
  key: string
  dotClass: 'high' | 'medium' | 'low' | 'idle'
  text: string
  date: string
  time: string
  overdue: boolean
  dim: boolean
}

/** 轨道行：条目映射为行；多于一条时末尾追加第一条副本（原型 items[0] 再拼一次）。 */
const rows = computed<IslandRow[]>(() => {
  const list = items.value
  if (!list.length) {
    return [{ key: 'empty', dotClass: 'idle', text: emptyText.value, date: '', time: '', overdue: false, dim: true }]
  }
  const mapped = list.map<IslandRow>((item) => ({
    key: item.id,
    dotClass: item.priority ?? 'idle',
    text: item.title,
    date: item.date ?? '',
    // 原型：逾期时时间前缀「逾期 」。
    time: item.overdue ? `逾期 ${item.meta ?? ''}`.trimEnd() : (item.meta ?? ''),
    overdue: Boolean(item.overdue),
    dim: false,
  }))
  return mapped.length > 1 ? [...mapped, { ...mapped[0], key: `${mapped[0].key}:clone` }] : mapped
})

// ── 几何：行高 = 胶囊折叠高；展开时撑到 max(高, 120) ──

/** 原型 DI.H_MIN / H_MAX / W_MIN / W_MAX（与 Rust island_clamp 一致）。 */
const LIMITS = { wMin: 200, wMax: 480, hMin: 32, hMax: 56 } as const
/** 悬停展开高度下限（与 Rust ISLAND_EXPANDED_HEIGHT 一致）。 */
const EXPANDED_MIN_HEIGHT = 120
/** 胶囊高度 CSS 过渡时长（extensions.css §10 的 0.32s），收起时据此延后缩窗。 */
const EXPAND_MS = 320
/** 原型 diStep：切换过渡 .45s，到副本后 480ms 复位。 */
const STEP_MS = 450
const RESET_MS = 480

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value))
}

/** 拖拽中的临时高度；null = 用设置值。 */
const dragHeight = ref<number | null>(null)
const itemH = computed(() => dragHeight.value ?? clamp(settings.value.island_height || 36, LIMITS.hMin, LIMITS.hMax))
const expanded = ref(false)
const capH = computed(() => (expanded.value ? Math.max(itemH.value, EXPANDED_MIN_HEIGHT) : itemH.value))
/** 背景不透明度：只透底色，文字始终不透明（D32 保留）。 */
const alpha = computed(() => clamp(settings.value.island_opacity || 0.85, 0.3, 1))

const rootStyle = computed<CSSProperties>(() => ({
  '--island-alpha': String(alpha.value),
  '--cap-h': `${capH.value}px`,
}))

// ── 逐条停留轮播（原型 diStep）──

const diIndex = ref(0)
const trackStyle = ref<CSSProperties>({ transition: 'none', transform: 'translateY(0)' })
let stepTimer: ReturnType<typeof setTimeout> | null = null
let resetTimer: ReturnType<typeof setTimeout> | null = null

const stayMs = computed(() => clamp(settings.value.island_cycle_seconds || 3, 1, 10) * 1000)
/** 轮播是否应该跑：多于一条，且没有展开 / 提醒 / 面板遮挡。 */
const panelOpen = ref(false)
const alert = ref<string | null>(null)
const canCycle = computed(() => items.value.length > 1 && !expanded.value && alert.value === null && !panelOpen.value)

/** 只停步进计时器；有意不动 resetTimer，让进行中的副本→第一条无过渡复位照常完成。 */
function stopTicker(): void {
  if (stepTimer) clearTimeout(stepTimer)
  stepTimer = null
}

function scheduleStep(): void {
  stopTicker()
  if (!canCycle.value) return
  stepTimer = setTimeout(step, stayMs.value)
}

/** 原型 diStep：步进一行；到副本后过渡结束时无过渡复位到真正第一条。 */
function step(): void {
  stepTimer = null
  if (!canCycle.value) return
  const count = items.value.length
  diIndex.value += 1
  trackStyle.value = {
    transition: `transform ${STEP_MS}ms ease`,
    transform: `translateY(${-diIndex.value * itemH.value}px)`,
  }
  if (diIndex.value === count) {
    if (resetTimer) clearTimeout(resetTimer)
    resetTimer = setTimeout(() => {
      resetTimer = null
      diIndex.value = 0
      trackStyle.value = { transition: 'none', transform: 'translateY(0)' }
    }, RESET_MS)
  }
  scheduleStep()
}

/** 条目 / 停留时长变化：回到第一条重新计时（原型 renderIsland 重建轨道）。 */
function resetTicker(): void {
  stopTicker()
  if (resetTimer) clearTimeout(resetTimer)
  resetTimer = null
  diIndex.value = 0
  trackStyle.value = { transition: 'none', transform: 'translateY(0)' }
  logger.debug('island', `轮播重置 items=${items.value.length} stay=${stayMs.value}ms`)
  scheduleStep()
}

watch(() => items.value.map((item) => item.id).join('|'), resetTicker)
watch(stayMs, resetTicker)
// 主窗口设置页改了胶囊高度：行高变了，带着 translateY(-i*h) 的轨道会错位，回到第一条重排（整分支审查 R2）。
watch(() => settings.value.island_height, resetTicker)
watch(canCycle, (can) => {
  if (can) scheduleStep()
  else stopTicker()
})

// ── 提醒岛内呈现（原型 showIslandAlert / hideIslandAlert）──

const ALERT_MS = 6000
let alertTimer: ReturnType<typeof setTimeout> | null = null

function showAlert(content: string): void {
  if (alertTimer) clearTimeout(alertTimer)
  alert.value = content
  logger.info('island', `提醒岛内呈现：${content}`)
  alertTimer = setTimeout(hideAlert, ALERT_MS)
}

function hideAlert(): void {
  if (alertTimer) clearTimeout(alertTimer)
  alertTimer = null
  if (alert.value === null) return
  alert.value = null
}

/** 通知后端是否正在与胶囊内元素交互（失败只记日志，不影响交互本身）。 */
async function setInteracting(on: boolean): Promise<void> {
  try {
    await api.island.setInteracting(on)
  } catch (error) {
    logger.error('island', `设置交互旗标 ${on} 失败`, error)
  }
}

/** 提醒卡按下：先让后端暂停点击探测，否则这次 mousedown 会被当成点击胶囊呼出面板。 */
function onAlertDown(): void {
  void setInteracting(true)
}

/** 提醒卡松手：解除交互旗标（后端只认按下边沿，松手后即可恢复探测）。 */
function onAlertUp(): void {
  void setInteracting(false)
}

/** 提醒卡点击：关闭提醒并恢复轮播；再解除一次旗标兜底（mouseup 若落在卡外不会触发 onAlertUp）。 */
function onAlertClick(): void {
  hideAlert()
  void setInteracting(false)
}

// ── 悬停展开（后端推送；D33 悬停穿透守卫）──

const clicked = ref(false)
const current = computed<IslandItem | null>(() => items.value[diIndex.value % Math.max(items.value.length, 1)] ?? null)

/** 展开序号：async 期间用于「后来者优先」，避免快速进出导致窗口尺寸错乱。 */
let expandSeq = 0
/**
 * 最近一次**请求**的展开态。去重要比对它而不是 expanded.value：
 * expand(true) IPC 在途时 expanded 仍为 false，此时到来的 hover(false) 若按 expanded 比对会被当成「已收起」丢掉，
 * 结果窗口撑开后没人再收回去（整分支审查 R3）。
 */
let wanted = false

/**
 * 切换展开态。为消除窗口 set_size 的瞬时跳变：
 * - 展开：先把窗口撑到展开高（透明空间落在胶囊下方，不可见），再置 expanded → 胶囊用 CSS 平滑生长、详情淡入；
 * - 收起：先置 expanded=false → 胶囊 CSS 收缩，待动画结束再缩窗。
 */
async function setExpanded(next: boolean): Promise<void> {
  if (wanted === next) return
  wanted = next
  const seq = ++expandSeq
  try {
    if (next) {
      await api.island.expand(true)
      if (seq !== expandSeq) return
      expanded.value = true
    } else {
      expanded.value = false
      await new Promise((resolve) => setTimeout(resolve, EXPAND_MS))
      if (seq !== expandSeq) return
      await api.island.expand(false)
    }
  } catch (error) {
    logger.error('island', '切换展开状态失败', error)
  }
}

// ── 手柄拖拽（原型 diResizer）──

/** 拖拽起点与起始尺寸；null = 未在拖拽。 */
let drag: { x: number; y: number; w: number; h: number } | null = null
/** 拖拽中的目标宽度（逻辑像素）；窗口宽度由后端在松手后统一调整。 */
const dragWidth = ref<number | null>(null)

function startResize(event: MouseEvent): void {
  if (settings.value.island_click_through) return
  drag = {
    x: event.clientX,
    y: event.clientY,
    w: clamp(settings.value.island_width || 320, LIMITS.wMin, LIMITS.wMax),
    h: itemH.value,
  }
  // 回到第一条并清掉过渡：拖拽中行高在变，带着 translateY(-i*h) 的轨道会错位。
  resetTicker()
  // 先挂交互旗标再监听：后端下一轮（80ms）探测到左键按下时已知这不是点击胶囊。
  void setInteracting(true)
  document.addEventListener('mousemove', onResizeMove)
  document.addEventListener('mouseup', onResizeEnd)
  logger.debug('island', `开始拖拽手柄 w=${drag.w} h=${drag.h}`)
}

function onResizeMove(event: MouseEvent): void {
  if (!drag) return
  dragWidth.value = clamp(Math.round(drag.w + (event.clientX - drag.x) * 2), LIMITS.wMin, LIMITS.wMax)
  dragHeight.value = clamp(Math.round(drag.h + (event.clientY - drag.y)), LIMITS.hMin, LIMITS.hMax)
}

async function onResizeEnd(): Promise<void> {
  document.removeEventListener('mousemove', onResizeMove)
  document.removeEventListener('mouseup', onResizeEnd)
  if (!drag) return
  const width = dragWidth.value ?? drag.w
  const height = dragHeight.value ?? drag.h
  drag = null
  logger.info('island', `手柄拖拽结束 width=${width} height=${height}`)
  try {
    await api.island.resize(width, height)
  } catch (error) {
    logger.error('island', '落库灵动岛尺寸失败', error)
  } finally {
    // 设置已由后端广播回来，清掉临时值改用设置值；轨道按新行高重排。
    dragWidth.value = null
    dragHeight.value = null
    resetTicker()
    // 默认设置下手柄只在展开态可达（任何悬停都先展开），而 island_resize 只按折叠高摆窗：
    // 松手后若仍处于展开态，需再让后端按 max(h, 120) 重新摆放，否则窗口会缩到胶囊下面把展开区裁掉（整分支审查 I3）。
    if (expanded.value) {
      try {
        await api.island.expand(true)
      } catch (error) {
        logger.error('island', '拖拽结束后恢复展开高失败', error)
      }
    }
    // 落库完成（或失败）后再解除旗标，避免松手瞬间被后端当成点击。
    await setInteracting(false)
  }
}

// ── 事件订阅 ──

onMounted(() => {
  scheduleStep()
  void onAppEvent<boolean>(AppEvents.islandHover, (inside) => {
    // 后端在面板可见时一律视为「不在区内」，面板打开后的下一轮会先发一次 inside=false——
    // 若在这里无条件恢复，会把 panelShown 刚设的 di-fade 立刻撤销。
    // 只有 inside=true 才说明面板确实已收起（可见时不可能发 true），此时补一次淡出恢复（隐藏期间 panelHidden 可能丢）。
    if (inside) panelOpen.value = false
    if (settings.value.island_pass_hover) {
      logger.debug('island', '悬停穿透开启，忽略悬停事件')
      return
    }
    logger.debug('island', inside ? '光标进入' : '光标离开')
    void setExpanded(inside)
  })
  void onAppEvent(AppEvents.islandClick, () => {
    logger.info('island', '点击，面板已由后端呼出')
    clicked.value = true
    setTimeout(() => (clicked.value = false), 300)
  })
  void onAppEvent<boolean>(AppEvents.panelShown, () => {
    panelOpen.value = true
  })
  void onAppEvent(AppEvents.panelHidden, () => {
    panelOpen.value = false
  })
  void onAppEvent<string>(AppEvents.reminderFired, (id) => {
    const target = todos.value.find((todo) => todo.id === id)
    if (!target) {
      logger.warn('island', `提醒 ${id} 未在本地待办列表中，跳过岛内呈现`)
      return
    }
    showAlert(target.content)
  })
})

onBeforeUnmount(() => {
  stopTicker()
  if (resetTimer) clearTimeout(resetTimer)
  if (alertTimer) clearTimeout(alertTimer)
  document.removeEventListener('mousemove', onResizeMove)
  document.removeEventListener('mouseup', onResizeEnd)
})
</script>

<template>
  <div
    id="dynamicIsland"
    class="glass"
    :class="{
      'di-fade': panelOpen,
      'di-pass-click': settings.island_click_through,
      'di-glow': settings.island_glow,
      expanded,
      clicked,
    }"
    :style="rootStyle"
  >
    <!-- 收起态：原型轨道，JS 步进 translateY；展开态 v-if 隐藏轨道避免与 --cap-h 过渡冲突（spec §8） -->
    <div v-if="!expanded" class="di-ticker">
      <div class="di-track" :style="trackStyle">
        <div v-for="row in rows" :key="row.key" class="di-item" :style="{ height: itemH + 'px' }">
          <span class="di-dot" :class="row.dotClass" />
          <span class="di-text" :class="{ dim: row.dim }"
            ><span v-if="row.date" class="di-date">{{ row.date }}</span
            >{{ row.text }}</span
          >
          <span v-if="row.time" class="di-time" :class="{ ovd: row.overdue }">{{ row.time }}</span>
        </div>
      </div>
    </div>

    <!-- 提醒岛内呈现：覆盖轨道，点击或 6s 后恢复 -->
    <div
      v-if="alert !== null"
      class="di-alert"
      title="点击关闭提醒并恢复轮播"
      @mousedown="onAlertDown"
      @mouseup="onAlertUp"
      @click="onAlertClick"
    >
      <span class="di-alert-ico">⏰</span><span class="di-alert-text">提醒：{{ alert }}</span>
    </div>

    <!-- 右下角手柄（穿透模式不渲染：窗口整体 set_ignore_cursor_events，手柄本就收不到鼠标；
         展开态也渲染：任何悬停都会展开，否则手柄不可达） -->
    <div
      v-if="!settings.island_click_through"
      class="di-resizer"
      title="拖动调整胶囊大小（宽 200-480 · 高 32-56）"
      @mousedown.prevent="startResize"
    />

    <!-- 展开态：当前条目的详情组件（随胶囊生长淡入） -->
    <Transition name="island-fade">
      <div v-if="expanded" class="island-expanded">
        <component :is="current.detail" v-if="current?.detail" :item="current" />
        <div v-else class="island-detail">
          <div class="island-detail-title">{{ current?.title ?? emptyText }}</div>
        </div>
        <div class="island-hint">左键点击打开待办面板</div>
      </div>
    </Transition>
  </div>
</template>
