<script setup lang="ts">
import { onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import type { MindMapNode } from 'simple-mind-map'
import { colorList } from '../constants'
import { requireMindMap, useMindMap } from '../core/useMindMap'

/**
 * 节点标签样式面板（移植参考端 NodeTagStyle.vue）。
 *
 * 点击节点标签（库事件 node_tag_click(node, tag, index, el)）时在标签下方弹出：
 * 改文字 / 改填充色 / 删除该标签，均走 `SET_NODE_TAG` 命令重写整个 tag 数组。
 * 兼容字符串型与 {text, style} 对象型标签。Teleport 到 body；缩放/平移/点空白/展开时关闭。
 */
const ctx = useMindMap()
const { bus } = ctx

const show = ref(false)
const position = reactive<{ left: string; top: string }>({ left: '0', top: '0' })
const node = ref<MindMapNode | null>(null)
const index = ref(0)
const text = ref('')
const fill = ref('')

// svg.js 元素，仅调用 rbox；无类型声明用 any。
function open(
  target: MindMapNode,
  tag: unknown,
  i: number,
  el: { rbox: () => { x: number; y: number; width: number; height: number } },
): void {
  node.value = target
  index.value = i
  if (typeof tag === 'string') {
    text.value = tag
    fill.value = ''
  } else {
    const t = tag as { text: string; style?: { fill?: string } }
    text.value = t.text
    fill.value = t.style?.fill ?? ''
  }
  const { x, y, width, height } = el.rbox()
  const boxWidth = 260
  let left = x + width / 2 - boxWidth / 2
  if (left < 0) left = 0
  if (left + boxWidth > window.innerWidth) left = window.innerWidth - boxWidth
  position.left = left + 'px'
  position.top = y + height + 5 + 'px'
  show.value = true
}

function hide(): void {
  show.value = false
  node.value = null
  index.value = 0
  text.value = ''
  fill.value = ''
}

/** 重写第 index 个标签（兼容字符串→对象升级），走 SET_NODE_TAG。 */
function updateTagInfo(patch: { text?: string; style?: Record<string, string> }): void {
  if (!node.value) return
  const tagData = [
    ...((node.value.getData('tag') as Array<string | { text: string; style?: Record<string, string> }>) ?? []),
  ]
  let item = tagData[index.value]
  const obj = typeof item === 'string' ? { text: item, style: {} } : { ...item, style: { ...(item.style ?? {}) } }
  if (patch.text) obj.text = patch.text
  if (patch.style) Object.assign(obj.style, patch.style)
  tagData[index.value] = obj
  requireMindMap(ctx).execCommand('SET_NODE_TAG', node.value, tagData)
}

function updateText(): void {
  const value = text.value.trim()
  if (!value) return
  updateTagInfo({ text: value })
}
function updateFill(color: string): void {
  fill.value = color
  updateTagInfo({ style: { fill: color } })
}
function deleteTag(): void {
  if (!node.value) return
  const tagData = [...((node.value.getData('tag') as unknown[]) ?? [])]
  tagData.splice(index.value, 1)
  requireMindMap(ctx).execCommand('SET_NODE_TAG', node.value, tagData)
  hide()
}

const offs: Array<() => void> = []
onMounted(() => {
  offs.push(
    bus.on('node_tag_click', (target, tag, i, el) =>
      open(
        target as MindMapNode,
        tag,
        i as number,
        el as { rbox: () => { x: number; y: number; width: number; height: number } },
      ),
    ),
    bus.on('scale', hide),
    bus.on('translate', hide),
    bus.on('svg_mousedown', hide),
    bus.on('expand_btn_click', hide),
  )
})
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <Teleport to="body">
    <div v-show="show" class="mm-tagpanel mm-panel" :style="position" @click.stop>
      <div class="mm-tagpanel-row">
        <input
          v-model="text"
          class="mm-dialog-input"
          placeholder="标签文字"
          @blur="updateText"
          @keydown.stop
          @keyup.enter.stop="updateText"
        />
        <button type="button" class="mm-tagpanel-del" title="删除该标签" @click.stop="deleteTag">删除</button>
      </div>
      <div class="mm-tagpanel-colors">
        <button
          v-for="c in colorList"
          :key="c"
          type="button"
          class="mm-tagpanel-color"
          :class="{ active: fill === c, transparent: c === 'transparent' }"
          :style="c === 'transparent' ? undefined : { background: c }"
          @click="updateFill(c)"
        />
      </div>
    </div>
  </Teleport>
</template>
