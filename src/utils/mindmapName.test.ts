import test from 'node:test'
import assert from 'node:assert/strict'
import { normalizeMapName } from './mindmapName'

test('normalizeMapName：去首尾空白、折叠内部空白', () => {
  assert.equal(normalizeMapName('  项目   规划  '), '项目 规划')
})

test('normalizeMapName：裁剪到 32 字符', () => {
  assert.equal(normalizeMapName('一'.repeat(40)), '一'.repeat(32))
  // 恰好 32 字符：原样返回
  assert.equal(normalizeMapName('一'.repeat(32)), '一'.repeat(32))
  // 第 32 位落在空白上：截断后再 trim，不留尾随空格
  assert.equal(normalizeMapName('a'.repeat(31) + '  b'), 'a'.repeat(31))
})

test('normalizeMapName：空串与纯空白回退为「未命名导图」', () => {
  assert.equal(normalizeMapName(''), '未命名导图')
  assert.equal(normalizeMapName('   \n\t '), '未命名导图')
})
