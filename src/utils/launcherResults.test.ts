import test from 'node:test'
import assert from 'node:assert/strict'
import type { BrowserHistoryRow, LauncherHit } from '@/service/tauri'
import type { Note, Todo } from '@/typings/domain'
import { mergeLauncherResults, type LauncherScopes } from './launcherResults'

/** 五个范围开关默认全开，用例按需覆盖。 */
const ALL: LauncherScopes = { apps: true, notes: true, todos: true, calc: true, history: true }

function hit(partial: Partial<LauncherHit> & { name: string; path: string }): LauncherHit {
  return { id: 1, kind: 'app', score: 1, matched_keyword: '', ...partial }
}

function note(partial: Partial<Note> & { id: string; content: string }): Note {
  return {
    editor_mode: 'text',
    mindmap_data: null,
    tags: [],
    is_draft: false,
    pinned: false,
    archived_at: null,
    created_at: '2026-09-13T00:00:00+00:00',
    updated_at: '2026-09-13T00:00:00+00:00',
    ...partial,
  }
}

function todo(partial: Partial<Todo> & { id: string; content: string; due_at: string }): Todo {
  return {
    completed_at: null,
    status: 'open',
    remind_at: null,
    remind_offset_minutes: null,
    remind_desktop: true,
    remind_email: false,
    repeat_rule: null,
    remind_off: false,
    priority: 'medium',
    remark: '',
    parent_id: null,
    tags: [],
    created_at: partial.due_at,
    updated_at: partial.due_at,
    ...partial,
  }
}

function history(url: string, title: string): BrowserHistoryRow {
  return { url, title, visited_at: 1_700_000_000 }
}

/** 本地时区构造 RFC3339（与 island.test.ts 同款：避免断言依赖机器时区）。 */
function at(year: number, month: number, day: number, hour: number): string {
  return new Date(year, month - 1, day, hour, 0, 0, 0).toISOString()
}

test('mergeLauncherResults：空查询只保留后端常用项（笔记 / 待办 / 历史都不参与）', () => {
  const rows = mergeLauncherResults({
    apps: [hit({ name: 'Visual Studio Code', path: 'C:/vscode.exe' })],
    notes: [note({ id: 'n1', content: '会议纪要' })],
    todos: [todo({ id: 't1', content: '写周报', due_at: at(2026, 9, 13, 10) })],
    history: [history('https://a.example/', 'A')],
    query: '',
    scopes: ALL,
  })
  assert.deepEqual(
    rows.map((r) => r.id),
    ['app:C:/vscode.exe'],
  )
  assert.equal(rows[0].cat, '应用')
  assert.equal(rows[0].sub, 'C:/vscode.exe')
})

test('mergeLauncherResults：`=` 与「计算器 / calc」都产出首条计算器条目，普通查询不产出', () => {
  const base = { apps: [], notes: [], todos: [], history: [], scopes: ALL }
  for (const query of ['=', '= 1+2', '计算器', 'calc']) {
    const rows = mergeLauncherResults({ ...base, query })
    assert.equal(rows[0]?.id, 'calc', query)
    assert.deepEqual(rows[0].action, { type: 'calc' })
    assert.equal(rows[0].ico, '🧮')
    assert.equal(rows[0].cat, '计算器')
    assert.equal(rows[0].sub, '打开系统计算器')
  }
  assert.equal(mergeLauncherResults({ ...base, query: '记事本' }).length, 0)
})

test('mergeLauncherResults：顺序为 计算器 → 应用 / 命令 → 笔记 → 待办 → 历史', () => {
  const rows = mergeLauncherResults({
    apps: [
      hit({ id: 1, kind: 'app', name: 'Visual Studio Code', path: 'C:/vscode.exe' }),
      hit({ id: 2, kind: 'command', name: '🧮 计算器', path: 'cmd:calc' }),
    ],
    notes: [note({ id: 'n1', content: 'calc 笔记' })],
    todos: [todo({ id: 't1', content: 'calc 待办', due_at: at(2026, 9, 13, 10), priority: 'high' })],
    history: [history('https://calc.example/', '')],
    query: 'calc',
    scopes: ALL,
  })
  assert.deepEqual(
    rows.map((r) => r.cat),
    ['计算器', '应用', '命令', '笔记', '待办', '历史'],
  )
  // 命令名里的 emoji 拆进 .li-ico，名称不含 emoji、也没有 path 副文案。
  const command = rows[2]
  assert.equal(command.ico, '🧮')
  assert.equal(command.name, '计算器')
  assert.equal(command.sub, undefined)
  // 待办副文案 = 归属日 · 优先级。
  assert.equal(rows[4].sub, '2026-09-13 · 优先级 高')
  // 历史：标题为空（后端没抓到 title）时回退 URL 当名称。
  assert.equal(rows[5].name, 'https://calc.example/')
})

test('mergeLauncherResults：范围开关逐项关闭后对应分类消失', () => {
  const input = {
    apps: [hit({ kind: 'app' as const, name: 'Code', path: 'C:/code.exe' })],
    notes: [note({ id: 'n1', content: 'code 笔记' })],
    todos: [todo({ id: 't1', content: 'code 待办', due_at: at(2026, 9, 13, 10) })],
    history: [history('https://code.example/', 'code')],
    query: 'code',
  }
  const cases: { scopes: LauncherScopes; gone: string }[] = [
    { scopes: { ...ALL, apps: false }, gone: '应用' },
    { scopes: { ...ALL, notes: false }, gone: '笔记' },
    { scopes: { ...ALL, todos: false }, gone: '待办' },
    { scopes: { ...ALL, calc: false }, gone: '计算器' },
    { scopes: { ...ALL, history: false }, gone: '历史' },
  ]
  for (const { scopes, gone } of cases) {
    const rows = mergeLauncherResults({ ...input, scopes })
    const cats = rows.map((r) => r.cat)
    assert.ok(!cats.includes(gone), `关闭「${gone}」后仍出现：${cats.join(',')}`)
    // 其他分类仍在。
    assert.ok(cats.length > 0, `关闭「${gone}」后什么都不剩`)
  }
  // 计算器开关关掉后 `=` 查询不再有计算器条目。
  assert.deepEqual(
    mergeLauncherResults({
      apps: [],
      notes: [],
      todos: [],
      history: [],
      query: '=',
      scopes: { ...ALL, calc: false },
    }),
    [],
  )
})

test('mergeLauncherResults：笔记按正文 / 标签 / 导图节点文本命中，导图用 🧠 且取前 8 条', () => {
  const notes = [
    ...Array.from({ length: 9 }, (_, i) => note({ id: `n${i}`, content: `项目会议 ${i}` })),
    note({ id: 'tag', content: '无关正文', tags: ['项目'] }),
    note({
      id: 'map',
      content: '',
      editor_mode: 'mindmap',
      mindmap_data: JSON.stringify({
        data: { text: '项目规划' },
        children: [{ data: { text: '里程碑' } }],
      }),
    }),
  ]
  const rows = mergeLauncherResults({
    apps: [],
    notes,
    todos: [],
    history: [],
    query: '项目',
    scopes: ALL,
  })
  const noteRows = rows.filter((r) => r.cat === '笔记')
  // 9 条正文命中被限流到 8 条，标签与导图不算进这 8 条（它们排在其后）。
  assert.equal(noteRows.filter((r) => r.id.startsWith('note:n')).length, 8)
  // 分档顺序与 NOTE_HIT_ORDER 一致（正文 → 标签 → 导图），笔记段总数 8 + 1 + 1。
  // 断言完整 id 序列而非 some / find：否则将来有人改回「三合一后 slice(0, 8)」，
  // 标签 / 导图被挤掉时只会挂一条 some，看不出顺序也看不出被截断。
  assert.deepEqual(
    noteRows.map((r) => r.id),
    ['note:n0', 'note:n1', 'note:n2', 'note:n3', 'note:n4', 'note:n5', 'note:n6', 'note:n7', 'note:tag', 'note:map'],
  )
  assert.equal(noteRows.length, 10, '笔记段应为 8（正文）+ 1（标签）+ 1（导图）')
  assert.ok(noteRows.some((r) => r.id === 'note:tag'))
  const map = noteRows.find((r) => r.id === 'note:map')
  assert.ok(map, '导图节点文本应命中')
  assert.equal(map.ico, '🧠')
  assert.equal(map.name, '思维导图: 项目规划')
  assert.deepEqual(map.action, { type: 'note', id: 'map', mindmap: true })
})
