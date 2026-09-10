import test from 'node:test'
import assert from 'node:assert/strict'
import { highlight, mindmapAllText, mindmapRootText } from './search'

test('highlight：忽略大小写，只标首个命中', () => {
  assert.deepEqual(highlight('Hello world hello', 'HELLO'), [
    { text: 'Hello', hit: true },
    { text: ' world hello', hit: false },
  ])
  assert.deepEqual(highlight('abc', ''), [{ text: 'abc', hit: false }])
  assert.deepEqual(highlight('abc', 'x'), [{ text: 'abc', hit: false }])
  assert.deepEqual(highlight('xyz', 'xyz'), [{ text: 'xyz', hit: true }])
  assert.deepEqual(highlight('前缀命中后缀', '命中'), [
    { text: '前缀', hit: false },
    { text: '命中', hit: true },
    { text: '后缀', hit: false },
  ])
})

test('mindmapAllText：全量格式与旧格式都递归，剥富文本标签', () => {
  const full = JSON.stringify({
    root: {
      data: { text: '<p>根</p>' },
      children: [{ data: { text: '子1' }, children: [{ data: { text: '孙' }, children: [] }] }],
    },
  })
  assert.equal(mindmapAllText(full), '根 子1 孙')
  const legacy = JSON.stringify({ data: { text: 'A' }, children: [{ data: { text: 'B' }, children: [] }] })
  assert.equal(mindmapAllText(legacy), 'A B')
  assert.equal(mindmapAllText(''), '')
  assert.equal(mindmapAllText('{not json'), '')
  assert.equal(mindmapAllText(null), '')
})

test('mindmapRootText：根节点文字，缺失回退', () => {
  assert.equal(
    mindmapRootText(JSON.stringify({ root: { data: { text: '<p>减脂计划</p>' }, children: [] } })),
    '减脂计划',
  )
  assert.equal(mindmapRootText(JSON.stringify({ root: { data: { text: '  ' }, children: [] } })), '未命名导图')
  assert.equal(mindmapRootText(null), '未命名导图')
  assert.equal(mindmapRootText(null, '中心主题'), '中心主题')
})
