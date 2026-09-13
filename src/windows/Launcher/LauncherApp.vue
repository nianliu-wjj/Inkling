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
 *   各受检索范围开关控制）；页脚两态按原型（`docs/app.js` renderLauncherResults）：有结果
 *   「↑↓ 导航 · ↵ 执行 · Esc 关闭 · 共 N 项」、无结果「↵ 回车搜索 · Esc 退出」，
 *   有结果时右侧再并入动作芯片 / 快捷键提示（D40，原型无此段）；
 * - 键盘：↑↓ 循环（选中项 `scrollIntoView`，原型 app.js:3326）、Enter 执行（无结果 → 默认浏览器搜索）、
 *   Tab 切动作、Alt+1..9 直达、Ctrl+Enter 管理员、Esc 隐藏——监听器挂 window 而非输入框：
 *   点击条目后焦点会离开输入框，挂 window 才能继续响应键盘（现有实现即如此）；
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

const { query, results, active, actions, actionIndex, reset, runActive, refresh, onKeydown } = useLauncherResults({
  scope: 'launcher',
  floating: true,
})

const root = ref<HTMLElement | null>(null)
const input = ref<HTMLInputElement | null>(null)
const list = ref<HTMLElement | null>(null)

/** 页脚左侧（原型 renderLauncherResults 的两态文案，`docs/app.js:3289` / `:3300`）。 */
const footerHint = computed(() =>
  results.value.length ? `↑↓ 导航 · ↵ 执行 · Esc 关闭 · 共 ${results.value.length} 项` : '↵ 回车搜索 · Esc 退出',
)

/**
 * 空态文案：有查询词用原型的「未找到匹配项，按 Enter 用默认浏览器搜索「X」」（`docs/app.js:3289`）；
 * 空查询且无结果（索引未建好、或关掉了「应用与命令」范围）原型到不了这一态，
 * 用自造的「索引尚未就绪」——比显示一对空引号「」诚实。与启动台页 `LauncherPageView` 同一口径。
 */
const emptyHint = computed(() =>
  query.value.trim() ? `未找到匹配项，按 Enter 用默认浏览器搜索「${query.value.trim()}」` : '索引尚未就绪',
)

/**
 * 页脚右侧的动作芯片（D40）：只有应用 / 文件 / 文件夹这类多动作条目才有可选动作，
 * 其余条目（命令 / UWP / 笔记 / 待办 / 计算器 / 历史）芯片留空。
 * 芯片本身就是「当前动作」的可见反馈，因此尾巴里不必再重复「当前动作：」前缀
 * ——那是 11px 字号下页脚折行的主因（560px 窗口两栏合计上限 516px）。
 */
const currentAction = computed(() =>
  actions.value.length < 2 ? '' : (actions.value[actionIndex.value]?.label ?? '打开'),
)

/**
 * 页脚右侧的键位提示（D40，原型无此段）。
 * 无结果时整段不显示——原型无结果页脚只有左侧一段（`docs/app.js:3289`），
 * 且此时组合式的 `actions` 仍是 `[打开]`（长度 1 而非空数组），照下面分支会紧挨着空列表
 * 提示一个没有任何条目可跳的「Alt+1..9 直达」。
 * 单动作条目省略 Tab 与 Ctrl+Enter 段——它们没有管理员 / 定位动作，提示了也执行不了（后端会直接报错）。
 */
const actionsHint = computed(() => {
  if (!results.value.length) return ''
  if (actions.value.length < 2) return 'Alt+1..9 直达'
  return 'Tab 切换 · Ctrl+Enter 管理员 · Alt+1..9'
})

/** 点击条目：默认「打开」。 */
function run(index: number): void {
  void runActive(index)
}

/**
 * 选中项滚动进视野（原型 ↑↓ 分支的 `scrollIntoView({ block: 'nearest' })`，`docs/app.js:3326`）。
 * 订阅 `active` 而不是写在键盘处理里：键盘在组合式内（`useLauncherResults.onKeydown`），这里只能订阅结果。
 * 鼠标悬停同样会改 `active`，但悬停项本就在视野内，`nearest` 不会产生位移。
 * 只滚 `.launcher-list` 这一个最近的可滚动祖先——窗口与 body 都是 overflow: hidden，不会带动整页。
 */
watch(active, () => {
  void nextTick(() => {
    if (!results.value.length) return
    list.value?.children[active.value]?.scrollIntoView({ block: 'nearest' })
  })
})

/**
 * 换查询时列表回到顶部（2026-09-13 实机验收补丁，见 4C 验收记录 §3）。
 *
 * 原型每次输入都重建 `innerHTML`（`docs/app.js:3310`），滚动位置天然归零；
 * 我们用的是 keyed `v-for`，节点复用会**保留 scrollTop**，而 `search()` 把 `active`
 * 复位为 0 时上面的 `watch(active)` 并不触发（值没变）——于是出现「选中项是第 0 条
 * （回车执行它），视野里却停在一屏看不到它的中间行」。
 */
watch(query, () => {
  void nextTick(() => {
    if (list.value) list.value.scrollTop = 0
  })
})

/** 窗口每次显示：清空、聚焦、重拉缓存、播入场（原型 openLauncher）。 */
function onFocus(): void {
  reset()
  // 隐藏期间 notes/todos/settings 的变更事件会丢（WebView2 挂起），呼出时重拉三份缓存。
  // reset() 触发的空查询检索与 refresh() 里的重复一次无所谓：本地索引 IPC，毫秒级。
  void refresh()
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

    <div ref="list" class="launcher-list">
      <div
        v-for="(result, index) in results"
        :key="result.id"
        class="launcher-item"
        :class="{ active: index === active }"
        @mouseenter="active = index"
        @click="run(index)"
      >
        <span class="li-ico">{{ result.ico }}</span>
        <span class="li-name"
          >{{ result.name }}<small v-if="result.sub">{{ result.sub }}</small></span
        >
        <span class="li-cat">{{ result.cat }}</span>
      </div>
      <div v-if="!results.length" class="launcher-empty">{{ emptyHint }}</div>
    </div>

    <div class="launcher-footer">
      <span class="launcher-nav">{{ footerHint }}</span>
      <!-- 无结果（actionsHint 为空串）时整段不渲染：留一个空 span 也会占掉 footer 的 12px gap。 -->
      <span v-if="actionsHint" class="launcher-keys"
        ><span v-if="currentAction" class="launcher-act">{{ currentAction }}</span
        >{{ actionsHint }}</span
      >
    </div>
  </div>
</template>
