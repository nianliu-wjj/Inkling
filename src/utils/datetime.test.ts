import test from 'node:test'
import assert from 'node:assert/strict'
import { formatDateKey, formatDueLabel, toDateKey } from './datetime'

test('formatDueLabel：今天带「今天」，其余一律 M/D HH:mm（无昨天 / 明天）', () => {
  const now = new Date()
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate(), 9, 5)
  assert.equal(formatDueLabel(today.toISOString()), '今天 09:05')
  const tomorrow = new Date(today.getTime() + 86_400_000)
  assert.equal(formatDueLabel(tomorrow.toISOString()), `${tomorrow.getMonth() + 1}/${tomorrow.getDate()} 09:05`)
  const yesterday = new Date(today.getTime() - 86_400_000)
  assert.equal(formatDueLabel(yesterday.toISOString()), `${yesterday.getMonth() + 1}/${yesterday.getDate()} 09:05`)
  assert.equal(formatDueLabel('not-a-date'), '')
})

test('formatDateKey：原样 YYYY-MM-DD，可选「 · 今天」后缀', () => {
  const todayKey = toDateKey(new Date())
  assert.equal(formatDateKey('2026-01-05'), '2026-01-05')
  assert.equal(formatDateKey(todayKey, { todaySuffix: true }), `${todayKey} · 今天`)
  assert.equal(formatDateKey('2000-01-01', { todaySuffix: true }), '2000-01-01')
})
