import test from 'node:test'
import assert from 'node:assert/strict'
import { anchorBeside, type AnchorOptions } from './anchor'

/** 1280×800 视口、200×60 浮层：绝大多数用例共用。 */
function opts(partial: Partial<AnchorOptions> = {}): AnchorOptions {
  return {
    size: { width: 200, height: 60 },
    viewport: { width: 1280, height: 800 },
    align: 'center',
    fallback: 'center',
    ...partial,
  }
}

test('anchorBeside：右侧放得下 → 卡片右缘 +14，垂直居中，箭头对齐卡片中心', () => {
  const result = anchorBeside({ left: 100, top: 300, width: 400, height: 100 }, opts())
  assert.equal(result.placement, 'right')
  assert.equal(result.left, 514) // 100 + 400 + 14
  assert.equal(result.top, 320) // 300 + 50 - 30
  assert.equal(result.caretY, 30) // 350 - 320
})

test('anchorBeside：右侧不够 → 翻到左侧（卡片左缘 −14 − W）', () => {
  const result = anchorBeside({ left: 1000, top: 300, width: 200, height: 100 }, opts())
  // 右侧 1200 + 14 + 200 = 1414 > 1270；左侧 1000 − 14 − 200 = 786 ≥ 10
  assert.equal(result.placement, 'left')
  assert.equal(result.left, 786)
  assert.equal(result.top, 320)
})

test('anchorBeside：两侧都不够，fallback=center → 水平居中盖住卡片（原型）', () => {
  const result = anchorBeside(
    { left: 20, top: 100, width: 440, height: 80 },
    opts({ viewport: { width: 480, height: 600 } }),
  )
  // 右侧 460 + 14 + 200 = 674 > 470；左侧 20 − 214 < 10
  assert.equal(result.placement, 'center')
  assert.equal(result.left, 140) // (480 − 200) / 2
  assert.equal(result.top, 110) // 100 + 40 − 30
})

test('anchorBeside：两侧都不够，fallback=below → 卡片下方 +8、左缘对齐；下方放不下则改到上方', () => {
  const below = anchorBeside(
    { left: 20, top: 100, width: 440, height: 80 },
    opts({ viewport: { width: 480, height: 600 }, fallback: 'below' }),
  )
  assert.equal(below.placement, 'below')
  assert.equal(below.left, 20)
  assert.equal(below.top, 188) // 100 + 80 + 8
  // 视口极窄时左缘超出可放范围 → left 钳制到 vw − W − 10
  const clamped = anchorBeside(
    { left: 60, top: 100, width: 100, height: 80 },
    opts({ viewport: { width: 230, height: 600 }, fallback: 'below' }),
  )
  // 右侧 160 + 214 > 220；左侧 60 − 214 < 10；下方 left = clamp(60, 10, 20)
  assert.equal(clamped.placement, 'below')
  assert.equal(clamped.left, 20)
  // 卡片贴底：下方 + 8 放不下 → 放到卡片上方 −8
  const above = anchorBeside(
    { left: 20, top: 520, width: 440, height: 70 },
    opts({ viewport: { width: 480, height: 600 }, fallback: 'below' }),
  )
  assert.equal(above.placement, 'below')
  assert.equal(above.top, 452) // 520 − 60 − 8
})

test('anchorBeside：align=top 对齐卡片顶部，且 top 钳制在 [10, vh − H − 10]', () => {
  const top = anchorBeside({ left: 100, top: 300, width: 400, height: 100 }, opts({ align: 'top' }))
  assert.equal(top.top, 300)
  // 卡片在视口顶部以上 → 钳到 10
  const clampedTop = anchorBeside({ left: 100, top: -40, width: 400, height: 100 }, opts({ align: 'top' }))
  assert.equal(clampedTop.top, 10)
  // 卡片贴底 → 钳到 800 − 60 − 10
  const clampedBottom = anchorBeside({ left: 100, top: 790, width: 400, height: 100 }, opts({ align: 'top' }))
  assert.equal(clampedBottom.top, 730)
})

test('anchorBeside：caretY 钳制在 [14, H − 14]；无锚点则居中且 placement=center', () => {
  // 卡片贴顶（top 钳到 10），卡片中心 15 → 箭头 5 → 钳到 14
  const low = anchorBeside({ left: 100, top: -20, width: 400, height: 70 }, opts())
  assert.equal(low.caretY, 14)
  // 卡片贴底（top 钳到 730），卡片中心 795 → 箭头 65 → 钳到 46
  const high = anchorBeside({ left: 100, top: 760, width: 400, height: 70 }, opts())
  assert.equal(high.caretY, 46)
  const centered = anchorBeside(null, opts())
  assert.deepEqual(centered, { left: 540, top: 370, placement: 'center', caretY: 30 })
})
