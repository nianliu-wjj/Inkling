import test from 'node:test'
import assert from 'node:assert/strict'
import type { ActivityDay } from '@/typings/domain'
import { MINI_THRESHOLDS, STATS_THRESHOLDS, alphaOf, buildMonthGrid, buildRangeGrid, levelOf } from './heatmap'

const day = (date: string, notes = 0, clips = 0, todos = 0, overdue = 0): ActivityDay => ({
  date,
  notes,
  clips,
  todos,
  completed: 0,
  overdue,
})

test('levelOf：迷你 <3/<6/<10，统计 <5/<10/<18；alphaOf 五档', () => {
  assert.deepEqual(
    [0, 1, 2, 3, 5, 6, 9, 10].map((n) => levelOf(n, MINI_THRESHOLDS)),
    [0, 1, 1, 2, 2, 3, 3, 4],
  )
  assert.deepEqual(
    [0, 4, 5, 9, 10, 17, 18].map((n) => levelOf(n, STATS_THRESHOLDS)),
    [0, 1, 2, 2, 3, 3, 4],
  )
  assert.deepEqual(
    [0, 1, 2, 3, 4].map((l) => alphaOf(l as 0 | 1 | 2 | 3 | 4)),
    [0, 0.18, 0.38, 0.6, 0.88],
  )
})

test('buildMonthGrid：周一对齐，2026-09-01 是周二 → 1 个空位；共 5 列 35 格', () => {
  const cells = buildMonthGrid([day('2026-09-10', 2, 0, 1, 1)], 2026, 8, { selected: '2026-09-10' })
  assert.equal(cells.length, 35)
  assert.equal(cells[0].blank, true)
  assert.equal(cells[1].key, '2026-09-01')
  const target = cells.find((c) => c.key === '2026-09-10')
  assert.ok(target)
  assert.equal(target.level, 2)
  assert.equal(target.overdue, true)
  assert.equal(target.selected, true)
  assert.equal(cells[30].key, '2026-09-30')
  assert.equal(cells[31].blank, true)
  assert.equal(cells[34].blank, true)
})

test('buildRangeGrid：182 天起点回退到周一；月份标签去重且间距 ≥3 列', () => {
  const today = new Date(2026, 8, 10) // 周四
  const { cells, months } = buildRangeGrid([day('2026-09-10', 20)], { days: 182, today })
  // 今天 − 181 天 = 2026-03-13（周五）→ 回退到周一 2026-03-09
  assert.equal(cells[0].key, '2026-03-09')
  assert.equal(cells.at(-1)?.key, '2026-09-10')
  assert.equal(cells.at(-1)?.level, 4)
  assert.deepEqual(
    months.map((m) => m.label),
    ['3月', '4月', '5月', '6月', '7月', '8月', '9月'],
  )
  for (let i = 1; i < months.length; i += 1) assert.ok(months[i].column - months[i - 1].column >= 3)
  assert.equal(months[0].column, 0)
})
