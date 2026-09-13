<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useSettings } from '@/composables/useData'
import { useLauncherResults } from '@/composables/useLauncherResults'
import { applyCachedGlass, useGlass } from '@/composables/useGlass'
import { applyCachedTheme, useTheme } from '@/composables/useTheme'
import { enter } from '@/motion'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'

/**
 * 浮窗启动台（原型 `#launcherWindow`，spec 4C §4.2）。
 *
 * - DOM 按原型：`.glass > .launcher-input-row(.launcher-ico + #launcherInput) + .launcher-list + .launcher-footer`；
 * - 结果源与主窗口启动台页共用 `useLauncherResults`（应用 / 命令 / 笔记 / 待办 / 计算器 / 浏览器历史，
 *   各受检索范围开关控制）；页脚两态：有结果「↑↓ 导航 · ↵ 执行 · 点击直接执行 · 共 N 项」、
 *   无结果「↵ 回车搜索 · Esc 退出」，右侧并入动作 / 快捷键提示（D40）；
 * - 键盘：↑↓ 循环、Enter 执行（无结果 → 默认浏览器搜索）、Tab 切动作、Alt+1..9 直达、
 *   Ctrl+Enter 管理员、Esc 隐藏——监听器挂 window 而非输入框：点击条目后焦点会离开输入框，
 *   挂 window 才能继续响应键盘（现有实现即如此）；
 * - 每次显示（window focus）：清空查询、选中归零、聚焦输入框、播入场（y −18 + 淡入 + scale .98）；
 * - 失焦即隐藏（点了别处）——与原型「点击窗外关闭」能力等价。
 */
applyCachedTheme()
applyCachedGlass()
document.documentElement.dataset.window = 'launcher'

const { settings } = useSettings()
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

const { query, results, active, actions, actionIndex, reset, runActive, onKeydown } = useLauncherResults({
  scope: 'launcher',
  floating: true,
})

const root = ref<HTMLElement | null>(null)
const input = ref<HTMLInputElement | null>(null)

/** 页脚左侧（原型 renderLauncherResults 的两态文案）。 */
const footerHint = computed(() =>
  results.value.length ? `↑↓ 导航 · ↵ 执行 · 点击直接执行 · 共 ${results.value.length} 项` : '↵ 回车搜索 · Esc 退出',
)

/**
 * 页脚右侧（D40，原型无此段）：当前动作 + 键位提示。
 * 命令 / UWP 等只有「打开」的条目省略 Tab 与 Ctrl+Enter 段——它们没有管理员 / 定位动作，
 * 提示了也执行不了（后端会直接报错）。
 */
const actionsHint = computed(() => {
  if (actions.value.length < 2) return 'Alt+1..9 直达'
  const current = actions.value[actionIndex.value]?.label ?? '打开'
  return `当前动作：${current} · Tab 切换动作 · Alt+1..9 直达 · Ctrl+Enter 管理员`
})

/** 点击条目：默认「打开」。 */
function run(index: number): void {
  void runActive(index)
}

/** 窗口每次显示：清空、聚焦、播入场（原型 openLauncher）。 */
function onFocus(): void {
  reset()
  void nextTick(() => input.value?.focus())
  if (root.value) void enter(root.value, { axis: 'y', distance: -18, scale: 0.98, duration: 'base' })
  logger.debug('launcher', '浮窗显示：已清空输入并聚焦')
}

/** 失焦即隐藏（点了别处）。 */
function onBlur(): void {
  void api.launcher.hide()
}

onMounted(() => {
  window.addEventListener('keydown', onKeydown)
  window.addEventListener('focus', onFocus)
  window.addEventListener('blur', onBlur)
  onFocus()
  logger.info('launcher', '启动台浮窗已挂载')
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
  window.removeEventListener('focus', onFocus)
  window.removeEventListener('blur', onBlur)
})
</script>

<template>
  <div id="launcherWindow" ref="root" class="glass">
    <div class="launcher-input-row">
      <span class="launcher-ico">🚀</span>
      <input
        id="launcherInput"
        ref="input"
        v-model="query"
        type="text"
        placeholder="搜索应用 / 命令 / 笔记 / 待办 / 历史，= 开头打开计算器"
        spellcheck="false"
        autocomplete="off"
      />
    </div>

    <div class="launcher-list">
      <div
        v-for="(result, index) in results"
        :key="result.id"
        class="launcher-item"
        :class="{ active: index === active }"
        @mousemove="active = index"
        @click="run(index)"
      >
        <span class="li-ico">{{ result.ico }}</span>
        <span class="li-name"
          >{{ result.name }}<small v-if="result.sub">{{ result.sub }}</small></span
        >
        <span class="li-cat">{{ result.cat }}</span>
      </div>
      <div v-if="!results.length" class="launcher-empty">
        未找到匹配项，按 Enter 用默认浏览器搜索「{{ query.trim() }}」
      </div>
    </div>

    <div class="launcher-footer">
      <span>{{ footerHint }}</span>
      <span class="launcher-keys">{{ actionsHint }}</span>
    </div>
  </div>
</template>
