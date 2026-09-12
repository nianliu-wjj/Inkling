import test from 'node:test'
import assert from 'node:assert/strict'
import { parsePanelIntent } from './panelIntent'

test('parsePanelIntent：只有 page 时 noteId 为 undefined', () => {
  assert.deepEqual(parsePanelIntent('{"page":"todo"}'), { page: 'todo', noteId: undefined })
})

test('parsePanelIntent：page 与 noteId 都解析出来', () => {
  assert.deepEqual(parsePanelIntent('{"page":"note","noteId":"abc"}'), { page: 'note', noteId: 'abc' })
})

test('parsePanelIntent：noteId 非字符串时丢弃，page 保留', () => {
  assert.deepEqual(parsePanelIntent('{"page":"note","noteId":42}'), { page: 'note', noteId: undefined })
})

test('parsePanelIntent：非法输入一律视为无意图', () => {
  assert.equal(parsePanelIntent(null), null)
  assert.equal(parsePanelIntent(undefined), null)
  assert.equal(parsePanelIntent(''), null)
  assert.equal(parsePanelIntent('{not json'), null)
  assert.equal(parsePanelIntent('"todo"'), null)
  assert.equal(parsePanelIntent('42'), null)
  assert.equal(parsePanelIntent('null'), null)
  assert.equal(parsePanelIntent('[]'), null)
  assert.equal(parsePanelIntent('{}'), null)
  assert.equal(parsePanelIntent('{"noteId":"abc"}'), null)
  assert.equal(parsePanelIntent('{"page":1}'), null)
})
