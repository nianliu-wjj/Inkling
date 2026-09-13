import { computed, onBeforeUnmount, ref, watch, type ComputedRef, type Ref } from 'vue'
import { useNotes, useSettings, useTodos } from '@/composables/useData'
import { logger } from '@/service/logger'
import { api, type BrowserHistoryRow, type LauncherHit } from '@/service/tauri'
import { mergeLauncherResults, type LauncherResult, type LauncherScopes } from '@/utils/launcherResults'

/**
 * 启动台共享结果源（浮窗启动台与主窗口启动台页共用，spec 4C §4.1）。
 *
 * 取数：查询去抖 30ms → `api.launcher.search`（应用 / 命令 / 文件）与
 * `api.browserHistory.search`（仅历史开关打开且查询词非空时；否则清空本地结果，
 * 开关重新打开时补一次查询）；
 * 笔记 / 待办来自 `useNotes` / `useTodos` 的全量列表（随数据变更事件自动刷新），
 * 怎么合并交给纯函数 `mergeLauncherResults`。
 *
 * 执行：`runActive` 按条目动作分派——launch 走后端启动（mode 由 Tab / Ctrl+Enter 决定）、
 * note 先收起浮窗再开导图 / 回显面板、todos 打开主窗口待办页、calc 走 `cmd:calc`、
 * url 用系统默认浏览器打开。
 *
 * 键盘：`floating: true`（浮窗）额外启用 Esc 隐藏、Tab 循环动作、Alt+1..9 直达；
 * 启动台页只保留 ↑↓ 与 Enter（原型行为），同时不抢走 Tab 的焦点移动。
 */

/** 可直接执行的动作；key 同时是后端 `LaunchMode` 的字面量。 */
export type LauncherActionKey = 'open' | 'admin' | 'reveal'

/** 动作栏的一项。 */
export interface LauncherActionItem {
  key: LauncherActionKey
  label: string
}

/** 动作全集；只有「应用 / 文件 / 文件夹」支持后两项，其余条目只给「打开」。 */
const ACTIONS: LauncherActionItem[] = [
  { key: 'open', label: '打开' },
  { key: 'admin', label: '管理员' },
  { key: 'reveal', label: '打开所在文件夹' },
]

/** 输入去抖（ms）：输入即搜，但避免每个按键都打一次 IPC。 */
const DEBOUNCE_MS = 30

/** 无结果时回车改用默认浏览器搜索（原型 launcher-empty 的提示语）。 */
const SEARCH_FALLBACK_URL = 'https://www.google.com/search?q='

export interface UseLauncherResults {
  query: Ref<string>
  results: ComputedRef<LauncherResult[]>
  active: Ref<number>
  /** 当前条目可用的动作（Tab 循环）。 */
  actions: ComputedRef<LauncherActionItem[]>
  actionIndex: Ref<number>
  /** 清空查询并把选中归零（浮窗每次显示调用）。 */
  reset: () => void
  move: (delta: number) => void
  cycleAction: () => void
  runActive: (index?: number, mode?: LauncherActionKey) => Promise<void>
  /** 强制重拉模块级缓存并重跑一次检索（浮窗每次呼出调用）。 */
  refresh: () => Promise<void>
  onKeydown: (event: KeyboardEvent) => void
}

export function useLauncherResults(options: { scope?: string; floating?: boolean } = {}): UseLauncherResults {
  const scope = options.scope ?? 'launcher'
  // 浮窗专属键位 + 无结果回车后收起浮窗；启动台页两者都不做（原型页面行为）。
  const floating = options.floating ?? false
  // 三份缓存都是模块级单例，这里额外取出各自的 reload 供浮窗呼出时强制刷新（见下面的 refresh）。
  const { settings, reload: reloadSettings } = useSettings()
  const { notes, reload: reloadNotes } = useNotes()
  const { todos, reload: reloadTodos } = useTodos()

  const query = ref('')
  const active = ref(0)
  const actionIndex = ref(0)
  /** 后端应用 / 命令 / 文件命中。 */
  const apps = ref<LauncherHit[]>([])
  /** 后端浏览器历史命中（历史开关关闭时恒为空）。 */
  const history = ref<BrowserHistoryRow[]>([])
  let debounce: ReturnType<typeof setTimeout> | null = null
  /** 组件卸载后迟到的响应不再写 ref（窗口/页面已销毁）。 */
  let disposed = false

  async function search(): Promise<void> {
    const keyword = query.value
    // 空查询不搜历史：后端 `LIKE '%%'` 会白取回一批行，纯函数又会整段丢掉；
    // 历史条目只在有查询词时出现（原型同）。历史开关关闭同样走 else（本地结果清空）。
    const withHistory = settings.value.launcher_scope_history && keyword.trim() !== ''
    try {
      if (withHistory) {
        // 两路并行：历史走自有表，与应用检索互不依赖。
        const [hits, rows] = await Promise.all([api.launcher.search(keyword), api.browserHistory.search(keyword)])
        apps.value = hits
        history.value = rows
      } else {
        apps.value = await api.launcher.search(keyword)
        history.value = []
      }
      active.value = 0
      actionIndex.value = 0
      logger.debug(scope, `检索「${keyword}」命中 ${apps.value.length + history.value.length} 条后端结果`)
    } catch (error) {
      logger.error(scope, '检索失败', error)
      apps.value = []
      history.value = []
    }
  }

  /**
   * 强制刷新：重拉笔记 / 待办 / 设置三份模块级缓存，再重跑一次当前查询。
   *
   * 浮窗被 `hide()` 后 WebView2 会挂起，`notesChanged` / `todosChanged` / `settingsChanged`
   * 事件在隐藏期间全部丢失（见 `useData.ts` 的模块级缓存 + `initialized` 守卫，
   * 以及 `PanelApp.vue` 对同类问题的处理），而浮窗是本项目唯一「长期隐藏复用 + 缓存型数据源」
   * 的窗口——不刷新就会一直停在首次呼出的快照上：隐藏期间新建的笔记搜不到，
   * 设置页刚关掉的检索范围也照样出结果。
   */
  async function refresh(): Promise<void> {
    await Promise.all([reloadNotes(), reloadTodos(), reloadSettings()])
    await search()
  }

  watch(
    query,
    () => {
      if (debounce) {
        clearTimeout(debounce)
        debounce = null
      }
      // 空查询（挂载 / 呼出清空）立即取常用项，不走 30ms 去抖：
      // 否则上一轮的应用结果会在入场动画期间继续显示，要等去抖 + 一次 IPC 往返才被替换。
      if (!query.value.trim()) {
        if (disposed) return
        void search()
        return
      }
      debounce = setTimeout(() => {
        // 卸载时同步清掉已排队的定时器，这里的兜底只防「定时器已触发但组件刚卸载」。
        if (disposed) return
        void search()
      }, DEBOUNCE_MS)
    },
    { immediate: true },
  )

  // 历史开关重新打开时补一次查询（关闭期间不发请求，本地结果已清空）。
  watch(
    () => settings.value.launcher_scope_history,
    (on) => {
      if (on) void search()
    },
  )

  const scopes = computed<LauncherScopes>(() => ({
    apps: settings.value.launcher_scope_apps,
    notes: settings.value.launcher_scope_notes,
    todos: settings.value.launcher_scope_todos,
    calc: settings.value.launcher_scope_calc,
    history: settings.value.launcher_scope_history,
  }))

  const results = computed<LauncherResult[]>(() =>
    mergeLauncherResults({
      apps: apps.value,
      notes: notes.value,
      todos: todos.value,
      history: history.value,
      query: query.value,
      scopes: scopes.value,
    }),
  )

  // 结果集变短时把选中拉回范围内（例如刚关掉某个检索范围）。本 watch 先于下面的 id watch 创建，
  // 同一次 flush 内也先执行，因此 id watch 读到的已是钳制后的 active（越界时不会多复位一次动作）。
  watch(results, (list) => {
    if (active.value >= list.length) active.value = Math.max(0, list.length - 1)
  })

  // 动作复位按「当前条目 id」变化触发，而不是按结果数组身份：results 是 computed，每次重算都返回新数组，
  // 按数组身份复位会让数据刷新打断已选动作（浮窗里在某应用条目上 Tab 切到「管理员」，
  // 此刻另一个窗口保存笔记 → actionIndex 悄悄回 0 → 回车执行的是「打开」）。
  watch(
    () => results.value[active.value]?.id,
    () => {
      if (actionIndex.value > 0) actionIndex.value = 0
    },
  )

  const actions = computed<LauncherActionItem[]>(() => {
    const action = results.value[active.value]?.action
    const multi = action?.type === 'launch' && action.kind !== 'command' && action.kind !== 'uwp'
    return multi ? ACTIONS : ACTIONS.slice(0, 1)
  })

  function reset(): void {
    query.value = ''
    active.value = 0
    actionIndex.value = 0
  }

  function move(delta: number): void {
    if (!results.value.length) return
    active.value = (active.value + delta + results.value.length) % results.value.length
    actionIndex.value = 0
  }

  function cycleAction(): void {
    if (actions.value.length < 2) return
    actionIndex.value = (actionIndex.value + 1) % actions.value.length
  }

  async function runActive(index = active.value, mode: LauncherActionKey = 'open'): Promise<void> {
    const result = results.value[index]
    if (!result) return
    logger.info(scope, `执行 ${result.id}`)
    try {
      switch (result.action.type) {
        case 'launch': {
          // 只有应用 / 文件 / 文件夹支持管理员与定位，其余条目忽略传入的 mode。
          const kind = result.action.kind
          const actionable = kind === 'app' || kind === 'file' || kind === 'folder'
          await api.launcher.launch(result.action.path, kind, actionable ? mode : 'open', query.value)
          return
        }
        case 'note':
          // 先收起浮窗再开导图 / 回显面板，避免启动台压在新窗口上（原型 closeLauncher）。
          await api.launcher.hide()
          if (result.action.mindmap) await api.windows.mindmapOpen(result.action.id)
          else await api.windows.panelOpenNote(result.action.id)
          return
        case 'todos':
          await api.launcher.hide()
          await api.windows.showMain('todos')
          return
        case 'calc':
          // 不内置表达式求值（D38）：后端拉起系统计算器。
          await api.launcher.launch('cmd:calc', 'command', 'open', query.value)
          return
        case 'url':
          // 用系统默认浏览器打开；启动台靠失焦自动隐藏（不显式 hide，避免与浏览器抢焦点打架）。
          await api.system.openUrl(result.action.url)
          return
        default: {
          // 兜底分支：将来给 `LauncherAction` 加第 6 种 `type` 却忘了在这里分派时，
          // 编译期由穷尽断言挡住（那时 `result.action` 不再是 `never`，这一行报错），
          // 运行期再记一条日志——本仓库没有 eslint、tsconfig 也没开 noImplicitReturns，
          // 光靠人眼审查的话，漏掉的类型会表现成「Enter 静默无反应」且无迹可查。
          const unhandled: never = result.action
          logger.error(scope, '未知的动作类型，已忽略', unhandled)
          return
        }
      }
    } catch (error) {
      logger.error(scope, '执行失败', error)
    }
  }

  /**
   * 键盘：↑↓ 循环、Enter 执行（无结果 → 默认浏览器搜索该词，原型行为）；
   * 浮窗额外支持 Tab 循环动作、Alt+1..9 直达、Ctrl+Enter 管理员、Esc 隐藏。
   */
  function onKeydown(event: KeyboardEvent): void {
    // 中文输入法：Enter 用于确认候选词（组合期间 isComposing 为真，部分环境只有 keyCode 229），
    // 此时不应触发执行或导航。
    if (event.isComposing || event.keyCode === 229) return
    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault()
        move(1)
        return
      case 'ArrowUp':
        event.preventDefault()
        move(-1)
        return
      case 'Tab':
        if (!floating) return
        event.preventDefault()
        cycleAction()
        return
      case 'Enter': {
        event.preventDefault()
        if (!results.value.length) {
          const keyword = query.value.trim()
          if (keyword) {
            logger.info(scope, '无结果，改用默认浏览器搜索')
            void api.system.openUrl(`${SEARCH_FALLBACK_URL}${encodeURIComponent(keyword)}`)
            if (floating) void api.launcher.hide()
          }
          return
        }
        // Ctrl+Enter 只在多动作条目（应用 / 文件 / 文件夹）上强制管理员：命令 / UWP 等
        // 单动作条目没有管理员模式，硬传下去后端必然报错，故与「打开」等价
        // ——页脚也正因此对它们省略该提示。
        // `floating` 守卫与 Tab / Esc / Alt+N 一致：页面不接动作键（原型页面键盘分支只有 ↑↓ 与 Enter），
        // 否则在启动台页对应用 / 文件按 Ctrl+Enter 会静默走管理员模式、拉起 UAC。
        const mode: LauncherActionKey =
          event.ctrlKey && floating && actions.value.length > 1
            ? 'admin'
            : (actions.value[actionIndex.value]?.key ?? 'open')
        void runActive(active.value, mode)
        return
      }
      case 'Escape':
        if (!floating) return
        event.preventDefault()
        void api.launcher.hide()
        return
      default:
        // Alt+1..9 直达（D40）：只执行第 N 条的「打开」，不把选中项挪过去——执行后浮窗
        // 随即收起，移动选中没有可观察效果（旧实现会先 active = index）。
        if (floating && event.altKey && /^[1-9]$/.test(event.key)) {
          const index = Number(event.key) - 1
          if (index < results.value.length) {
            event.preventDefault()
            void runActive(index, 'open')
          }
        }
    }
  }

  onBeforeUnmount(() => {
    disposed = true
    if (debounce) clearTimeout(debounce)
  })

  return { query, results, active, actions, actionIndex, reset, move, cycleAction, runActive, refresh, onKeydown }
}
