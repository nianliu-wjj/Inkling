import test from 'node:test'
import assert from 'node:assert/strict'
import {
  parseCss,
  stringify,
  splitSections,
  isDeniedSelector,
  filterDenied,
  rootTokenDuplicates,
} from './css-split.mjs'

test('parseCss：注释 / 规则 / 嵌套 @media / 字符串内的花括号', () => {
  const css = `/* 头 */\n.a, .b { color: red; --x: url("data:a{b}"); }\n@media (x) { .c { top: 0 } }\n@keyframes k { from { opacity: 0 } to { opacity: 1 } }`
  const nodes = parseCss(css)
  assert.equal(nodes.length, 4)
  assert.deepEqual(nodes[0], { type: 'comment', text: '/* 头 */' })
  assert.equal(nodes[1].type, 'rule')
  assert.deepEqual(nodes[1].selectors, ['.a', '.b'])
  assert.equal(nodes[1].body, 'color: red; --x: url("data:a{b}");')
  assert.equal(nodes[2].type, 'at')
  assert.equal(nodes[2].children[0].selectors[0], '.c')
  assert.equal(nodes[3].type, 'atraw')
})

test('stringify 后再 parse 得到同样的结构', () => {
  const css = `.a { color: red }\n@media (x) { .c { top: 0 } }\n@keyframes k { from { opacity: 0 } }`
  const once = parseCss(css)
  const twice = parseCss(stringify(once))
  assert.deepEqual(twice, once)
})

test('isDeniedSelector：任一复合选择器命中拒绝名单即为演示脚手架', () => {
  assert.equal(isDeniedSelector('#demoBar'), true)
  assert.equal(isDeniedSelector('[data-theme="typewriter"] #demoBar'), true)
  assert.equal(isDeniedSelector('#menubar:hover .x'), true)
  assert.equal(isDeniedSelector('.dot.red'), true)
  assert.equal(isDeniedSelector('.onboard-title'), true)
  assert.equal(isDeniedSelector('.fake-app .x'), true)
  assert.equal(isDeniedSelector('.nav-dot.dot-note'), false)
  assert.equal(isDeniedSelector('.page-title'), false)
  assert.equal(isDeniedSelector('#hotzone'), false)
  assert.equal(isDeniedSelector('.di-dot.high'), false)
})

test('filterDenied：逗号列表只删命中项，整条命中则丢弃，递归 @media', () => {
  const nodes = parseCss(
    `.ix, .onboard-title, #toast { filter: grayscale(1) }\n#demoBar { top: 0 }\n@media (x) { #menubar { a: b } .keep { a: b } }`,
  )
  const { nodes: kept, removed } = filterDenied(nodes)
  assert.equal(kept.length, 2)
  assert.deepEqual(kept[0].selectors, ['.ix', '#toast'])
  assert.equal(kept[1].type, 'at')
  assert.deepEqual(
    kept[1].children.map((n) => n.selectors[0]),
    ['.keep'],
  )
  assert.deepEqual(removed, ['.onboard-title', '#demoBar', '#menubar'])
})

test('splitSections：按标题注释切段，标记注释归入下一段，缺标记报错', () => {
  const css = `:root { --a: 1 }\n/* ═══ 全局质量层 ═══ */\n.q { a: b }\n/* ═══ 桌面环境 ═══ */\n#desktop { a: b }\n/* ═══ hotzone ═══ */\n.c { a: b }\n/* ═══ 多主题系统 ═══ */\n[data-theme="x"] { a: b }`
  const spec = [
    { name: 'tokens', until: '═══ 全局质量层' },
    { name: 'base', until: '═══ 桌面环境' },
    { name: null, until: '═══ hotzone ═══' },
    { name: 'components', until: '多主题系统' },
    { name: 'themes', until: null },
  ]
  const s = splitSections(parseCss(css), spec)
  assert.deepEqual(Object.keys(s), ['tokens', 'base', '__drop2', 'components', 'themes'])
  assert.equal(s.tokens.length, 1)
  assert.equal(s.base[0].type, 'comment')
  assert.equal(s.base[1].selectors[0], '.q')
  assert.equal(s.__drop2[1].selectors[0], '#desktop')
  assert.equal(s.components[1].selectors[0], '.c')
  assert.equal(s.themes[1].selectors[0], '[data-theme="x"]')
  assert.throws(() => splitSections(parseCss('.a { b: c }'), spec), /全局质量层/)
})

test('rootTokenDuplicates：:root 内重复定义的 -- 令牌', () => {
  const nodes = parseCss(`:root { --a: 1; --b: url("x;y"); --a: 2; color: red }\n.x { --a: 3 }`)
  assert.deepEqual(rootTokenDuplicates(nodes), ['--a'])
})
