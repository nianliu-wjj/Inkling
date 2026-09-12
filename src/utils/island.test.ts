import test from 'node:test'
import assert from 'node:assert/strict'
import type { Todo } from '@/typings/domain'
import { pickIslandTodos } from './island'

/** 本地时区构造 RFC3339：所有用例以 2026-09-12 为「今天」，当前时刻 12:00。 */
function at(year: number, month: number, day: number, hour: number, minute = 0): string {
  return new Date(year, month - 1, day, hour, minute, 0, 0).toISOString()
}
const TODAY = '2026-09-12'
const NOW = new Date(2026, 8, 12, 12, 0, 0, 0)

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

test('pickIslandTodos：scope=today 取当天与此前逾期的未完成（明天与已完成排除），逾期在前', () => {
  const rows = pickIslandTodos(
    [
      todo({ id: 'a', content: '今天 A', due_at: at(2026, 9, 12, 15) }),
      todo({ id: 'b', content: '明天 B', due_at: at(2026, 9, 13, 9) }),
      todo({ id: 'c', content: '昨天 C', due_at: at(2026, 9, 11, 9) }),
      todo({ id: 'd', content: '已完成 D', due_at: at(2026, 9, 12, 16), status: 'done' }),
    ],
    'today',
    TODAY,
    NOW,
  )
  assert.deepEqual(
    rows.map((row) => row.id),
    ['c', 'a'],
  )
  assert.equal(rows[0].overdue, true)
  assert.equal(rows[1].text, '今天 A')
  assert.equal(rows[1].date, TODAY)
  assert.equal(rows[1].time, '15:00')
  assert.equal(rows[1].overdue, false)
})

test('pickIslandTodos：scope=all 取全部未完成，逾期在前，其余按完成时间升序', () => {
  const rows = pickIslandTodos(
    [
      todo({ id: 'later', content: '明天', due_at: at(2026, 9, 13, 9) }),
      todo({ id: 'today', content: '今天下午', due_at: at(2026, 9, 12, 15) }),
      todo({ id: 'ovd-today', content: '今天早上已过', due_at: at(2026, 9, 12, 9) }),
      todo({ id: 'ovd-old', content: '昨天', due_at: at(2026, 9, 11, 18) }),
      todo({ id: 'done', content: '完成', due_at: at(2026, 9, 10, 9), status: 'done' }),
    ],
    'all',
    TODAY,
    NOW,
  )
  assert.deepEqual(
    rows.map((row) => row.id),
    ['ovd-old', 'ovd-today', 'today', 'later'],
  )
  assert.equal(rows[0].overdue, true)
  assert.equal(rows[0].date, '2026-09-11')
  assert.equal(rows[3].overdue, false)
})

test('pickIslandTodos：子任务文案为「父 / 子」，优先级取子任务自身；父已完成仍按子任务状态入选', () => {
  const rows = pickIslandTodos(
    [
      todo({ id: 'p', content: '写周报', due_at: at(2026, 9, 12, 18), priority: 'high', status: 'done' }),
      todo({ id: 'c1', content: '收集数据', due_at: at(2026, 9, 12, 14), parent_id: 'p', priority: 'low' }),
      todo({ id: 'c2', content: '孤儿子任务', due_at: at(2026, 9, 12, 16), parent_id: 'missing' }),
    ],
    'today',
    TODAY,
    NOW,
  )
  assert.deepEqual(
    rows.map((row) => [row.id, row.text, row.priority]),
    [
      ['c1', '写周报 / 收集数据', 'low'],
      ['c2', '孤儿子任务', 'medium'],
    ],
  )
})

test('pickIslandTodos：空集与全部完成返回空数组', () => {
  assert.deepEqual(pickIslandTodos([], 'today', TODAY, NOW), [])
  assert.deepEqual(
    pickIslandTodos(
      [todo({ id: 'x', content: '完成', due_at: at(2026, 9, 12, 9), status: 'done' })],
      'all',
      TODAY,
      NOW,
    ),
    [],
  )
})
