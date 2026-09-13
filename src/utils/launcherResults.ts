import type { BrowserHistoryRow, LauncherHit } from '@/service/tauri'
import type { Note, Priority, Todo } from '@/typings/domain'
import { dateKeyOf } from './datetime'
import { mindmapAllText, mindmapRootText } from './search'

/**
 * 启动台结果源纯函数（原型 app.js `collectLauncherResults`，spec 4C §4.1）。
 *
 * 顺序与原型一致：计算器 → 应用 / 命令 / 文件 → 笔记 → 待办 → 历史；空查询只保留
 * 后端返回的应用 / 命令段（常用项）。五个检索范围开关逐段短路。
 *
 * 不碰 DOM、不读全局状态——笔记 / 待办 / 历史都由调用方取好传进来，
 * 浮窗启动台与主窗口启动台页共用同一份实现，保证两处结果完全一致。
 *
 * 只引 `@/service/tauri` 的**类型**（`import type`，运行时被剥掉），
 * 因此本模块可以直接跑 `node:test`（tsconfig.test.json 不含 tauri 前端实现）。
 */

/** 一条结果要执行的动作；由 `useLauncherResults` 解释成实际调用。 */
export type LauncherAction =
  | { type: 'launch'; path: string; kind: LauncherHit['kind'] }
  | { type: 'note'; id: string; mindmap: boolean }
  | { type: 'todos' }
  | { type: 'calc' }
  | { type: 'url'; url: string }

/** 一条可展示、可执行的结果（原型 `{ ico, name, sub, cat, run }`）。 */
export interface LauncherResult {
  /** 稳定 key（`分类:标识`），模板 v-for 用。 */
  id: string
  /** emoji 图标（`.li-ico`）。 */
  ico: string
  name: string
  /** `.li-name > small` 副文案；空则不渲染。 */
  sub?: string
  /** `.li-cat` 分类文案（应用 / 文件 / 文件夹 / 命令 / 笔记 / 待办 / 计算器 / 历史）。 */
  cat: string
  action: LauncherAction
}

/** 检索范围开关（对应设置里的五个 `launcher_scope_*` 键）。 */
export interface LauncherScopes {
  apps: boolean
  notes: boolean
  todos: boolean
  calc: boolean
  history: boolean
}

export interface MergeInputs {
  /** 后端命中（应用 / UWP / 文件 / 文件夹 / 命令）。 */
  apps: LauncherHit[]
  /** 全量笔记（调用方已取；命中判定在前端做）。 */
  notes: Note[]
  todos: Todo[]
  /** 后端按当前查询取回的历史（范围关时为空数组）。 */
  history: BrowserHistoryRow[]
  query: string
  scopes: LauncherScopes
}

/** 每档命中的条数上限：原型全量拼接，这里限流避免长列表把小分类挤出视野。 */
const SECTION_LIMIT = 8
/** 名称截断长度（原型 `n.text.slice(0, 36)`）。 */
const NAME_LIMIT = 36

/** 命中类型 → 展示分类（`.li-cat`）。 */
const KIND_CAT: Record<LauncherHit['kind'], string> = {
  app: '应用',
  uwp: '应用',
  file: '文件',
  folder: '文件夹',
  command: '命令',
}

/** 命中类型 → 图标；命令的名称自带 emoji（`scan.rs::builtin_commands`），不套这一格。 */
const KIND_ICON: Record<LauncherHit['kind'], string> = {
  app: '🖥',
  uwp: '🧩',
  file: '📄',
  folder: '📁',
  command: '⚡',
}

/** 优先级文案（原型待办副文案 `优先级 ${t.priority}`）。 */
const PRIORITY_LABEL: Record<Priority, string> = { high: '高', medium: '中', low: '低' }

/**
 * 笔记的命中原因，同时决定同名次内的展示顺序：
 * 正文 → 标签 → 导图节点文本（正文命中最容易一次刷出几十条，排最前也最先被限流）。
 */
type NoteHitReason = 'content' | 'tag' | 'mindmap'

/** 笔记分段顺序（与 `NoteHitReason` 的声明顺序一致）。 */
const NOTE_HIT_ORDER: NoteHitReason[] = ['content', 'tag', 'mindmap']

/** 单行化并截断（原型 `n.text.slice(0, 36)`；带换行的正文会让条目高度跳动）。 */
function clip(text: string): string {
  const flat = text.replace(/\s+/g, ' ').trim()
  return flat.length > NAME_LIMIT ? `${flat.slice(0, NAME_LIMIT)}…` : flat
}

/**
 * 拆出内置命令名里的 emoji：后端按 spec §4.5 把 emoji 写进名称（`🧮 计算器`），
 * 而 `.li-ico` 是独立的图标位——不拆会出现「图标位空白 + 名称里再带一个 emoji」。
 * 名称不含空格时整串当作文案，图标退回 ⚡。
 */
function splitCommandName(raw: string): { ico: string; name: string } {
  const index = raw.indexOf(' ')
  if (index <= 0) return { ico: KIND_ICON.command, name: raw }
  return { ico: raw.slice(0, index), name: raw.slice(index + 1) }
}

/** 后端命中 → 结果条目。 */
function fromHit(hit: LauncherHit): LauncherResult {
  if (hit.kind === 'command') {
    // 命令的 path 是 `cmd:x` 这类标识，不作为副文案展示（原型命令条目的 sub 为空）。
    const { ico, name } = splitCommandName(hit.name)
    return {
      id: `command:${hit.path}`,
      ico,
      name,
      cat: KIND_CAT.command,
      action: { type: 'launch', path: hit.path, kind: hit.kind },
    }
  }
  return {
    id: `${hit.kind}:${hit.path}`,
    ico: KIND_ICON[hit.kind],
    name: hit.name,
    sub: hit.path,
    cat: KIND_CAT[hit.kind],
    action: { type: 'launch', path: hit.path, kind: hit.kind },
  }
}

/** 笔记命中判定：正文 → 标签 → 导图节点文本，取首个命中的原因；都不中返回 null。 */
function noteHitReason(note: Note, needle: string): NoteHitReason | null {
  if (note.content.toLowerCase().includes(needle)) return 'content'
  if (note.tags.some((tag) => tag.toLowerCase().includes(needle))) return 'tag'
  return mindmapAllText(note.mindmap_data).toLowerCase().includes(needle) ? 'mindmap' : null
}

/** 笔记 → 结果条目；导图笔记显示根节点文字与 🧠。 */
function fromNote(note: Note): LauncherResult {
  const mindmap = note.editor_mode === 'mindmap'
  const label = mindmap ? mindmapRootText(note.mindmap_data) : clip(note.content)
  return {
    id: `note:${note.id}`,
    ico: mindmap ? '🧠' : '📝',
    name: `${mindmap ? '思维导图' : '笔记'}: ${label}`,
    // 原型：有标签显示标签，否则显示时间；这里退化为空（不渲染 small）。
    sub: note.tags.length ? `[${note.tags.join(', ')}]` : '',
    cat: '笔记',
    action: { type: 'note', id: note.id, mindmap },
  }
}

/** 合并各数据源（原型 `collectLauncherResults`）。 */
export function mergeLauncherResults(input: MergeInputs): LauncherResult[] {
  const query = input.query.trim()
  const needle = query.toLowerCase()
  const out: LauncherResult[] = []

  // 计算器（D38）：`=` 开头或命中「计算器 / calc」时置顶，执行后端 `cmd:calc`（打开系统计算器）。
  if (input.scopes.calc && (query.startsWith('=') || /计算器|calc/i.test(query))) {
    out.push({
      id: 'calc',
      ico: '🧮',
      name: '计算器',
      sub: '打开系统计算器',
      cat: '计算器',
      action: { type: 'calc' },
    })
  }

  if (input.scopes.apps) out.push(...input.apps.map(fromHit))

  // 空查询到此为止：原型只在有查询词时才搜笔记 / 待办 / 历史。
  if (!query) return out

  if (input.scopes.notes) {
    // 按命中原因分档后各自限流：正文命中一次可能几十条，先切到 8 条；
    // 标签 / 导图节点是更精确的命中，排在其后且不占这 8 个名额（否则会被正文刷屏整段挤掉）。
    const buckets: Record<NoteHitReason, Note[]> = { content: [], tag: [], mindmap: [] }
    for (const note of input.notes) {
      const reason = noteHitReason(note, needle)
      if (reason) buckets[reason].push(note)
    }
    for (const reason of NOTE_HIT_ORDER) {
      out.push(...buckets[reason].slice(0, SECTION_LIMIT).map(fromNote))
    }
  }

  if (input.scopes.todos) {
    const matched = input.todos.filter((todo) => todo.content.toLowerCase().includes(needle))
    out.push(
      ...matched.slice(0, SECTION_LIMIT).map((todo): LauncherResult => ({
        id: `todo:${todo.id}`,
        ico: '✅',
        name: `待办: ${clip(todo.content)}`,
        sub: `${dateKeyOf(todo.due_at)} · 优先级 ${PRIORITY_LABEL[todo.priority]}`,
        cat: '待办',
        action: { type: 'todos' },
      })),
    )
  }

  if (input.scopes.history) {
    out.push(
      ...input.history.map((row): LauncherResult => ({
        id: `history:${row.url}`,
        ico: '🌐',
        name: row.title || row.url,
        sub: row.url,
        cat: '历史',
        action: { type: 'url', url: row.url },
      })),
    )
  }

  return out
}
