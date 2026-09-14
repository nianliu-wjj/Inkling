import test from 'node:test'
import assert from 'node:assert/strict'
import { normalizeMapName } from './mindmapName'

test('normalizeMapName：去首尾空白、折叠内部空白', () => {
  assert.equal(normalizeMapName('  项目   规划  '), '项目 规划')
})

test('normalizeMapName：裁剪到 32 字符', () => {
  const long = '一'.repeat(40)
  assert.equal(normalizeMapName(long).length, 32)
  // 32 字符以内原样返回
  assert.equal(normalizeMapName('短名字'), '短名字')
})

test('normalizeMapName：空串与纯空白回退为「未命名导图」', () => {
  assert.equal(normalizeMapName(''), '未命名导图')
  assert.equal(normalizeMapName('   \n\t '), '未命名导图')
})
