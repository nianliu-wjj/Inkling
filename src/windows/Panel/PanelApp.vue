<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  shallowReactive,
  watch,
  type ComponentPublicInstance,
} from 'vue'
import ToastHost from '@/components/base/ToastHost.vue'
import { useSettings } from '@/composables/useData'
import { applyCachedGlass, useGlass } from '@/composables/useGlass'
import { applyCachedTheme, useTheme } from '@/composables/useTheme'
import { AppEvents, onAppEvent } from '@/service/events'
import { logger } from '@/service/logger'
import { api, type PanelIntent } from '@/service/tauri'
import { MAX_HOTKEY_SLOTS, resolvePlugins, type PanelPageExpose, type PanelPlugin } from '@/panel-plugins'
import { enter, exit } from '@/motion'

/**
 * 呼出面板：由插件注册表驱动的多态容器。
 *
 * 页面不再硬编码——启用哪些、什么顺序由 `Settings.panel_plugins` 决定，
 * 组件从 `@/panel-plugins` 注册表解析（见该文件对「为何不做运行时加载」的说明）。
 * 新增一种捕获能力只需在注册表登记，不必改本文件。
 *
 * 需求 2.1：
 * - 入场 280ms outBack / 收起 180ms inQuad 的弹性过渡（animejs，与原型 showPanel/hidePanel 一致）；
 * - 固定宽 480px，高度随内容自适应（90~600px）；
 * - 失焦按设置策略收起：立即 / 延迟 3 秒 / 固定不收起；
 * - **弹窗失焦保护**：任一编辑弹窗打开期间不因失焦收起；全部关闭后若鼠标
 *   已不在面板内，按策略重新计时。
 * - Esc 收起；⌃1/2/3 切换三态。
 * - Zen 专注模式（spec D15）：窗口铺满工作区、#panel.zen-mode；Esc 只退 Zen；
 * - 回显编辑态（spec D16）：呼出意图带 noteId 时让笔记页 loadNote；收面板前通知各页 onPanelHide。
 */

// 启动瞬间先用缓存主题上色，避免默认深色闪一下再跳变。
applyCachedTheme()
applyCachedGlass()

const { settings } = useSettings()
const { applyTheme } = useTheme()
const { applyGlass } = useGlass()

/** 当前展示的插件 id；空串表示尚未初始化，由下方 watch 兜到首个插件。 */
const activeId = ref('')
const panel = ref<HTMLElement | null>(null)
/** 面板内弹窗的层数：>0 时禁止失焦收起。 */
const modalDepth = ref(0)
/** 各插件页实例（按插件 id）：回显 / 聚焦 / 关浮层 / 收面板通知都经它调用，只调用页面实现了的方法。 */
const pageRefs = shallowReactive<Record<string, PanelPageExpose | null>>({})
/** Zen 专注模式：为真时根元素带 .zen-mode，窗口已由后端放大到工作区。 */
const zen = ref(false)
/**
 * 前端认为面板是否处于显示中（Zen 入口显隐用）：panelShown 置 true，hide() 置 false。
 * 窗口是预建隐藏的，挂载时并不可见；初值 true 只是为了让首次 panelShown 之前的状态与「未收起」一致。
 */
const visible = ref(true)

/** 模板 `:ref` 回调：v-for 里的组件实例按插件 id 记账。 */
function setPageRef(id: string, instance: Element | ComponentPublicInstance | null): void {
  pageRefs[id] = instance as unknown as PanelPageExpose | null
}
/**
 * 独立编辑窗口（editor）是否打开。
 *
 * 待办新增 / 编辑用的是铺满屏幕的独立窗口，它一拿到焦点面板就会 blur；
 * 面板必须在此期间保持展开，否则用户填写时面板已经收起，关闭后无处可回。
 */
const externalEditorOpen = ref(false)
/**
 * 鼠标是否在面板窗口内，用于弹窗关闭后判断是否重新计时。
 * 编辑弹窗 Teleport 到 body 后不再是 #panel 的后代，因此按整个文档而非 #panel 判断，
 * 否则弹窗一盖住 #panel 就会触发 mouseleave，弹窗关闭后面板会被误判为「鼠标已离开」而收起。
 */
const pointerInside = ref(false)
/** 当前被高度观察器跟踪的弹窗元素，弹窗开合时重新同步。 */
const observedModals = new Set<HTMLElement>()

/** 失焦收起的延迟定时器。 */
let collapseTimer: ReturnType<typeof setTimeout> | null = null
/** 高度自适应的观察器。 */
let resizeObserver: ResizeObserver | null = null
/**
 * DOM 变化观察器。
 *
 * 各页数据是异步加载的，列表渲染出来时 ResizeObserver 未必回调（面板此刻可能还没显示，
 * WebView 处于挂起状态），高度就会停在「数据到达前」的值，底部内容被窗口边界切掉。
 * 结构变化必然经过 DOM，用它补一次上报。
 */
let mutationObserver: MutationObserver | null = null
/** 上报的合帧句柄：一次 DOM 批量变更只发一次 IPC。 */
let reportFrame: number | null = null

/** 面板高度范围，与 Rust 侧 windows.rs 的 PANEL_MIN/MAX_HEIGHT 保持一致。 */
const PANEL_MIN_HEIGHT = 90
const PANEL_MAX_HEIGHT = 600
/** 内容高度之外预留给窗口的余量（面板阴影与边框）。 */
const PANEL_HEIGHT_PADDING = 12

/** 当前启用的插件，顺序即展示顺序与快捷键序号。 */
const plugins = computed(() => resolvePlugins(settings.value.panel_plugins))

/** Zen 入口显隐（原型 updateZenToggle）：面板可见 ∧ 未处于 Zen ∧ 笔记页 ∧ 回显编辑态。 */
const zenAvailable = computed(
  () => visible.value && !zen.value && activeId.value === 'note' && (pageRefs.note?.isEditing ?? false),
)

/**
 * 进入 / 退出 Zen（原型 setZenMode）。
 * 进入：先让后端放大窗口再加类，避免 100vh 布局先在小窗口里闪一下；
 * 退出：先去类让内容收回，再让后端按记录的逻辑高度还原窗口。
 */
async function setZen(on: boolean): Promise<void> {
  if (zen.value === on) return
  logger.info('panel', on ? '进入 Zen 专注模式' : '退出 Zen 专注模式')
  if (!on) zen.value = false
  try {
    await api.windows.panelSetZen(on)
  } catch (error) {
    logger.error('panel', 'Zen 窗口切换失败', error)
    // 退出失败：后端语义上窗口仍是全屏，把提前去掉的类加回去，让状态与窗口一致；
    // 此时 zen.value 回到 true，下次再调 setZen(false) 不会被首行守卫挡住，可重试。
    // 进入失败：类尚未加上，保持 false 即可。
    if (!on) zen.value = true
    return
  }
  zen.value = on
  if (on) {
    // 原型：进入后 60ms 聚焦编辑器
    setTimeout(() => pageRefs[activeId.value]?.focus?.(), 60)
  } else {
    // Zen 期间高度上报被暂停，退出后按内容补报一次
    void nextTick(() => reportHeight())
  }
}

/** 切到指定插件页；页不存在或未启用时忽略（不让面板落到空白）。 */
function navigateTo(page: string | null | undefined): void {
  if (!page) return
  if (!plugins.value.some((plugin) => plugin.id === page)) {
    logger.warn('panel', `请求切到未启用的插件页 ${page}，忽略`)
    return
  }
  logger.info('panel', `切到插件页 ${page}`)
  activeId.value = page
}

/**
 * 保证 activeId 始终指向一个启用中的插件。
 *
 * 用户在设置里禁用了当前正在看的插件时也要落到有效页面上，
 * 否则面板会变成一片空白——没有任何 v-show 命中。
 */
watch(
  plugins,
  (list) => {
    if (!list.some((plugin) => plugin.id === activeId.value)) {
      activeId.value = list[0]?.id ?? ''
    }
  },
  { immediate: true },
)

/** 后端设置变化时同步主题。 */
watch(
  () => settings.value.theme,
  (theme) => applyTheme(theme),
  { immediate: true },
)

// 玻璃质感与主题同源：后端设置变化时一并同步。
watch(
  () => settings.value.glass_level,
  (level) => applyGlass(level),
  { immediate: true },
)

function clearCollapseTimer(): void {
  if (collapseTimer !== null) {
    clearTimeout(collapseTimer)
    collapseTimer = null
  }
}

/** 滑出动画结束后再真正隐藏窗口，避免窗口先消失、动画看不见。 */
function motionAxis(): 'x' | 'y' {
  return settings.value.panel_position === 'left' || settings.value.panel_position === 'right' ? 'x' : 'y'
}

function motionDistance(distance: number): number {
  const position = settings.value.panel_position
  return position === 'bottom' || position === 'right' ? distance : -distance
}

/**
 * 通知各页面板即将收起（笔记页据此丢弃未保存的回显修改并恢复草稿）。
 * hide() 与「补跑清理」（见 panelShown 处理）共用，保证两条路径的清理内容一致。
 */
function runPagesHide(): Promise<void> {
  return Promise.all(Object.values(pageRefs).map((page) => page?.onPanelHide?.())).then(() => undefined)
}

async function hide(): Promise<void> {
  clearCollapseTimer()
  logger.info('panel', '收起面板')
  // Zen 态先还原窗口（原型 hidePanel 的 setZenMode(false)）；Zen 下生成层 transform: none !important，位移动画无意义。
  if (zen.value) await setZen(false)

  if (panel.value) {
    // 与原型 hidePanel 一致：24px 位移 + 淡出，180ms inQuad。被新的入场打断时不再隐藏窗口。
    const completed = await exit(panel.value, { axis: motionAxis(), distance: motionDistance(24) })
    if (!completed) {
      logger.debug('panel', '收起动画被入场打断，取消隐藏')
      return
    }
  }
  // 收起即丢弃：通知各页。
  // 必须在窗口隐藏之前等它完成——WebView2 在窗口 hide 后挂起，此后的 IPC 回包要等下次显示。
  await runPagesHide()
  // 先置 false 再等后端：hide 后 WebView 可能挂起，await 之后的赋值未必及时执行；
  // 即使后端失败，下次 panelShown 也会重新置 true，状态不会卡死。
  visible.value = false
  try {
    await api.windows.panelHide()
  } catch (error) {
    logger.error('panel', '隐藏面板失败', error)
  }
}

/**
 * 入场：与原型 showPanel 一致——30px 位移 + 淡入 + 0.97 起始缩放，280ms outBack(1.6)。
 * 位移轴与方向按面板唤出位置推导（项目扩展，原型只有顶部）。
 * 由 onMounted 与后端 panelShown 事件触发，不依赖 CSS 动画自身的时间线（硬约束见 src/motion/presets.ts）。
 */
function playEnter(): void {
  if (!panel.value) return
  void enter(panel.value, { axis: motionAxis(), distance: motionDistance(30), scale: 0.97 })
}

/**
 * 按设置策略安排收起。
 * 弹窗打开期间直接跳过（弹窗失焦保护）。
 */
function scheduleCollapse(): void {
  if (modalDepth.value > 0) {
    logger.debug('panel', '弹窗打开中，跳过失焦收起')
    return
  }
  if (externalEditorOpen.value) {
    logger.debug('panel', '独立编辑窗口打开中，跳过失焦收起')
    return
  }

  const policy = settings.value.collapse_policy
  if (policy === 'never') return

  clearCollapseTimer()
  if (policy === 'immediate') {
    void hide()
    return
  }
  // 默认 3s
  collapseTimer = setTimeout(() => {
    collapseTimer = null
    void hide()
  }, 3000)
}

/**
 * 让高度观察器跟踪当前所有弹窗：弹窗在面板窗口里是整页编辑器，其内容增减
 * （添加标签、拖高备注等）同样需要驱动窗口高度变化。
 */
function syncModalObservers(): void {
  if (!resizeObserver) return
  const current = Array.from(document.querySelectorAll<HTMLElement>('.modal-shell'))
  for (const element of observedModals) {
    if (!current.includes(element)) {
      resizeObserver.unobserve(element)
      observedModals.delete(element)
    }
  }
  for (const element of current) {
    if (!observedModals.has(element)) {
      resizeObserver.observe(element)
      observedModals.add(element)
    }
  }
}

function onModalToggle(open: boolean): void {
  modalDepth.value = Math.max(0, modalDepth.value + (open ? 1 : -1))
  logger.debug('panel', `弹窗层数 = ${modalDepth.value}`)

  // 弹窗开合都会改变所需窗口高度：等 DOM 更新后重新跟踪弹窗并立即上报。
  void nextTick(() => {
    syncModalObservers()
    reportHeight()
  })

  if (open) {
    // 弹窗打开：取消已在计时的收起。
    clearCollapseTimer()
    return
  }
  // 全部弹窗关闭且鼠标已不在面板内 → 按策略重新计时。
  if (modalDepth.value === 0 && !pointerInside.value) scheduleCollapse()
}

/** 待办编辑窗口打开：取消已在计时的收起，并进入保护状态。 */
function onExternalEditorOpen(): void {
  externalEditorOpen.value = true
  clearCollapseTimer()
  logger.debug('panel', '独立编辑窗口已打开，暂停失焦收起')
}

/** 取走后端暂存的呼出意图：切页；带 noteId 则让目标页回显该笔记。 */
async function consumeIntent(): Promise<void> {
  let intent: PanelIntent | null
  try {
    intent = await api.windows.panelTakeIntent()
  } catch (error) {
    logger.error('panel', '读取呼出意图失败', error)
    return
  }
  if (!intent) return
  logger.info('panel', `呼出意图 page=${intent.page} noteId=${intent.noteId ?? '-'}`)
  navigateTo(intent.page)
  if (!intent.noteId) return
  // 切页后等一帧，确保目标页已渲染并挂上实例引用。
  await nextTick()
  const page = pageRefs[intent.page]
  if (!page?.loadNote) {
    logger.warn('panel', `插件页 ${intent.page} 不支持回显，忽略 noteId`)
    return
  }
  await page.loadNote(intent.noteId)
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    // 弹窗自己处理 Esc（ModalShell 在捕获阶段拦截），面板不抢。
    if (modalDepth.value > 0) return
    event.preventDefault()
    // 原型 Esc 链首位：Zen 态只退 Zen，不收面板。
    if (zen.value) {
      void setZen(false)
      return
    }
    void hide()
    return
  }

  // ⌃1..⌃9 按启用顺序切换插件；超出 9 个的插件只能用圆点点击。
  if (event.ctrlKey || event.metaKey) {
    const index = Number(event.key) - 1
    const list = plugins.value
    if (index >= 0 && index < Math.min(list.length, MAX_HOTKEY_SLOTS)) {
      event.preventDefault()
      activeId.value = list[index].id
    }
  }
}

/**
 * 高度自适应：把内容实际高度报给窗口，钳制在 90~600px。
 *
 * 弹窗打开期间，弹窗在面板窗口中以整页编辑器的形式铺满窗口并盖住 #panel
 * （见 window-fit.css），此时窗口高度应跟随弹窗而非 #panel；
 * 多层弹窗取最高者。弹窗自身的 max-height 已按 PANEL_MAX_HEIGHT 收口，
 * 超出部分在弹窗内部滚动，因此这里量到的 offsetHeight 不会形成反馈回路。
 */
/** 合帧上报：把同一帧内的多次观察器回调合并成一次 IPC。 */
function scheduleReport(): void {
  if (reportFrame !== null) return
  reportFrame = requestAnimationFrame(() => {
    reportFrame = null
    reportHeight()
  })
}

function reportHeight(): void {
  // Zen 态窗口铺满工作区，内容高度没有意义；退出 Zen 时由 setZen 补报。
  if (zen.value) return
  if (!panel.value) return

  const modals = Array.from(document.querySelectorAll<HTMLElement>('.modal-shell'))
  // 只能量 offsetHeight，**不能掺 scrollHeight**。
  //
  // 笔记页的编辑器容器带 flex: 1，会跟着窗口高度一起长；而 scrollHeight 又把
  // 这份被拉伸的高度算进来，于是「量得更高 → 上报 → 窗口更高 → 量得更高」
  // 形成正反馈，面板一路膨胀到上限，编辑区被拉成一大片空白。
  // （同理也不能量 #app：它是 height: 100vh，scrollHeight 恒 ≥ 窗口高度。）
  // 异步内容导致的高度变化由下方的 MutationObserver 负责补报。
  const measure = (element: HTMLElement): number => element.offsetHeight
  const contentHeight = modals.length ? Math.max(...modals.map(measure)) : measure(panel.value)
  const height = Math.min(PANEL_MAX_HEIGHT, Math.max(PANEL_MIN_HEIGHT, Math.ceil(contentHeight) + PANEL_HEIGHT_PADDING))

  void api.windows.panelResize(height).catch((error) => {
    logger.error('panel', '调整面板高度失败', error)
  })
}

function onPointerEnter(): void {
  pointerInside.value = true
}

function onPointerLeave(): void {
  pointerInside.value = false
}

onMounted(() => {
  window.addEventListener('keydown', onKeydown)
  window.addEventListener('blur', scheduleCollapse)
  // 重新获得焦点时取消待执行的收起。
  window.addEventListener('focus', clearCollapseTimer)
  document.documentElement.addEventListener('mouseenter', onPointerEnter)
  document.documentElement.addEventListener('mouseleave', onPointerLeave)

  if (panel.value) {
    resizeObserver = new ResizeObserver(scheduleReport)
    resizeObserver.observe(panel.value)
    mutationObserver = new MutationObserver(scheduleReport)
    mutationObserver.observe(panel.value, { childList: true, subtree: true, characterData: true })
  }

  playEnter()

  // 后端每次显示面板都会广播：重播入场动画，并取走呼出意图（切页 / 回显）。
  // 隐藏期间的事件会丢，所以意图存在后端、由前端主动拉。
  // payload：本次显示前面板是否处于隐藏态。快捷键切换 / 粘贴走的是后端直接 hide，
  // 不经过前端 hide()，此时 Zen 类与回显编辑态都还留在前端；隐藏期间 panelHidden
  // 事件未必送达（WebView2 挂起），所以改在下次显示时按 wasHidden 补跑清理。
  void onAppEvent<boolean>(AppEvents.panelShown, async (wasHidden) => {
    clearCollapseTimer()
    if (wasHidden) {
      // 前端仍认为在显示 / 在 Zen，说明这次隐藏没走前端 hide()，清理被漏掉了。
      const missedCleanup = visible.value || zen.value
      // Rust 侧 panel_hide 总会退出 Zen：只要经历过隐藏，前端的 Zen 态一定已失效。
      zen.value = false
      if (missedCleanup) {
        logger.warn('panel', '检测到后端直接收起（快捷键 / 粘贴），补跑前端收起清理')
        await runPagesHide()
      }
    }
    visible.value = true
    // Zen 态下生成层 transform: none !important，位移入场动画无意义，跳过。
    if (!zen.value) playEnter()
    await consumeIntent()
  })

  // 面板已可见时其他窗口请求切页（灵动岛点击、后续插件），直接响应。
  void onAppEvent<string>(AppEvents.panelNavigate, (page) => navigateTo(page))

  // 独立编辑窗口关闭 → 解除保护；若此时鼠标已不在面板上，按策略重新计时。
  void onAppEvent(AppEvents.editorClosed, () => {
    externalEditorOpen.value = false
    logger.debug('panel', '独立编辑窗口已关闭，恢复失焦收起')
    if (!pointerInside.value) scheduleCollapse()
  })

  logger.info('panel', '面板已挂载')
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
  window.removeEventListener('blur', scheduleCollapse)
  window.removeEventListener('focus', clearCollapseTimer)
  document.documentElement.removeEventListener('mouseenter', onPointerEnter)
  document.documentElement.removeEventListener('mouseleave', onPointerLeave)
  resizeObserver?.disconnect()
  mutationObserver?.disconnect()
  observedModals.clear()
  if (reportFrame !== null) cancelAnimationFrame(reportFrame)
  clearCollapseTimer()
})

const activeLabel = computed(() => plugins.value.find((plugin) => plugin.id === activeId.value)?.label ?? '')

/** 圆点 title：前 9 个带 ⌃N 序号（原型 `title="笔记 (⌃1)"`）。 */
function hotkeyTitle(plugin: PanelPlugin, index: number): string {
  return index < MAX_HOTKEY_SLOTS ? `${plugin.label} (⌃${index + 1})` : plugin.label
}
</script>

<template>
  <!-- modal-open：弹窗作为整页编辑器盖在面板之上时，把面板本体隐去，避免透过毛玻璃叠影。 -->
  <div
    id="panel"
    ref="panel"
    class="glass"
    :class="{ 'modal-open': modalDepth > 0, 'zen-mode': zen }"
    :aria-label="`Inkling 呼出面板 · ${activeLabel}`"
  >
    <!-- 插件圆点导航（原型 .nav-dots）：矢量圆由生成层 ::before 绘制，序号即 ⌃N 快捷键 -->
    <div class="panel-nav">
      <div class="nav-dots" role="tablist" aria-label="捕获模式切换">
        <button
          v-for="(plugin, index) in plugins"
          :key="plugin.id"
          type="button"
          class="nav-dot"
          :class="[plugin.dotClass, { active: activeId === plugin.id }]"
          :data-mode="plugin.id"
          role="tab"
          :aria-selected="activeId === plugin.id"
          :aria-label="`${plugin.label}模式`"
          :title="hotkeyTitle(plugin, index)"
          @click="navigateTo(plugin.id)"
        />
      </div>
      <!-- 原型 #zenToggle：只在回显编辑态出现 -->
      <button
        v-if="zenAvailable"
        id="zenToggle"
        type="button"
        class="btn ghost tiny"
        title="Zen 专注模式：全屏沉浸编辑（Esc 退出）"
        @click="setZen(true)"
      >
        <span class="ix">🧘</span> Zen
      </button>
      <span class="panel-hint">Esc 收起</span>
    </div>

    <!-- 插件页面：用 v-show 而非 v-if，保留各自状态（如笔记草稿、搜索关键词）。
         两个事件统一绑定——modal 是面板内弹窗开合，external-editor 是独立编辑窗口；
         不 emit 对应事件的插件不受影响。 -->
    <component
      :is="plugin.component"
      v-for="plugin in plugins"
      v-show="activeId === plugin.id"
      :key="plugin.id"
      :ref="(el: Element | ComponentPublicInstance | null) => setPageRef(plugin.id, el)"
      @modal="onModalToggle"
      @external-editor="onExternalEditorOpen"
      @zen-exit="setZen(false)"
    />

    <ToastHost />
  </div>
</template>
