import test from 'node:test'
import assert from 'node:assert/strict'
import type { Todo } from '@/typings/domain'
import { searchTodos } from './todo'

const todo = (id: string, content: string, due: string, extra: Partial<Todo> = {}): Todo => ({
  id,
  content,
  due_at: due,
  completed_at: null,
  status: 'open',
  remind_at: null,
  remind_offset_minutes: null,
  remind_desktop: false,
  remind_email: false,
  repeat_rule: null,
  remind_off: false,
  priority: 'medium',
  remark: '',
  parent_id: null,
  tags: [],
  created_at: due,
  updated_at: due,
  ...extra,
})

const data: Todo[] = [
  todo('p1', '写周报', '2026-09-10T02:00:00.000Z'),
  todo('c1', '整理数据', '2026-09-10T01:00:00.000Z', { parent_id: 'p1' }),
  todo('p2', '买牛奶', '2026-09-12T02:00:00.000Z', { priority: 'high' }),
  todo('c2', '写周报草稿', '2026-09-12T01:00:00.000Z', { parent_id: 'p2' }),
  todo('p3', '无关事项', '2026-09-11T02:00:00.000Z'),
]

test('searchTodos：命中父级 → 整棵树；只命中子任务 → 父级强制展开并标记命中子任务', () => {
  const result = searchTodos(data, '周报')
  assert.deepEqual(
    result.nodes.map((n) => n.todo.id),
    ['p2', 'p1'],
  ) // 完成日期降序
  assert.deepEqual([...result.forceExpand], ['p2'])
  assert.deepEqual([...result.hitIds], ['c2'])
  assert.equal(result.nodes[1].children.length, 1)
})

test('searchTodos：无命中或空查询都返回空', () => {
  assert.equal(searchTodos(data, '不存在').nodes.length, 0)
  const empty = searchTodos(data, '  ')
  assert.equal(empty.nodes.length, 0)
  assert.equal(empty.forceExpand.size, 0)
  assert.equal(empty.hitIds.size, 0)
})

test('searchTodos：同日期按优先级高在前', () => {
  const sameDay = [
    todo('a', '任务 甲', '2026-09-10T02:00:00.000Z', { priority: 'low' }),
    todo('b', '任务 乙', '2026-09-10T03:00:00.000Z', { priority: 'high' }),
  ]
  assert.deepEqual(
    searchTodos(sameDay, '任务').nodes.map((n) => n.todo.id),
    ['b', 'a'],
  )
})
