<script setup lang="ts">
import { transformToMarkdown } from 'simple-mind-map/src/parse/toMarkdown.js'
import { transformToTxt } from 'simple-mind-map/src/parse/toTxt.js'
import { getTextFromHtml, imgToDataUrl } from 'simple-mind-map/src/utils/index.js'
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import type { MindMapNode } from 'simple-mind-map'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { requireMindMap, useMindMap } from '../core/useMindMap'

/**
 * 右键菜单（移植参考端 Contextmenu.vue）。
 *
 * 两套菜单：节点菜单（type='node'）与画布菜单（type='svg'）。
 * 显示时机：
 * - 节点右键：库派发 `node_contextmenu(e, node)`；
 * - 画布右键：`svg_mousedown` 记右键按下点，`mouseup` 时若没拖动且不是在节点上按下，则显示画布菜单；
 * - `node_click / draw_click / expand_btn_click / translate` 都隐藏。
 * 命令经 `bus.emit('execCommand', …)` 或直接调库（copy/cut/paste/fit 等），与参考端一致。
 */
const ctx = useMindMap()
const { bus, ui, localConfig } = ctx
const { toast } = useToast()

const isShow = ref(false)
const left = ref(-9999)
const top = ref(-9999)
const type = ref<'node' | 'svg'>('node')
const node = ref<MindMapNode | null>(null)
const menuRef = ref<HTMLElement | null>(null)
const subLeft = ref(false)

// 右键按下→松开的判定（区分「右键点击」与「右键拖动画布」）。
let mousedownX = 0
let mousedownY = 0
let isMousedown = false
let isNodeMousedown = false

const canClipboardImg = typeof navigator !== 'undefined' && !!navigator.clipboard

const expandLevels = ['一级主题', '二级主题', '三级主题', '四级主题', '五级主题', '六级主题']
const copyList = computed(() => {
  const list = [
    { name: 'SMM', value: 'smm' },
    { name: 'JSON', value: 'json' },
    { name: 'Markdown', value: 'md' },
    { name: 'Txt', value: 'txt' },
  ]
  if (canClipboardImg) list.push({ name: '图片', value: 'png' })
  return list
})

const insertDisabled = computed(() => !node.value || node.value.isRoot || node.value.isGeneralization)
const upDisabled = computed(() => {
  const n = node.value
  if (!n || n.isRoot || n.isGeneralization) return true
  return n.parent?.children.indexOf(n) === 0
})
const downDisabled = computed(() => {
  const n = node.value
  if (!n || n.isRoot || n.isGeneralization) return true
  const children = n.parent?.children ?? []
  return children.indexOf(n) === children.length - 1
})
const hasHyperlink = computed(() => !!node.value?.getData('hyperlink'))
const hasNote = computed(() => !!node.value?.getData('note'))

function positionAt(x: number, y: number): void {
  const rect = menuRef.value?.getBoundingClientRect()
  if (rect) {
    if (x + rect.width > window.innerWidth) x = x - rect.width - 20
    subLeft.value = x + rect.width + 150 > window.innerWidth
    if (y + rect.height > window.innerHeight) y = window.innerHeight - rect.height - 10
  }
  left.value = x
  top.value = y
}

function showNode(e: MouseEvent, n: MindMapNode): void {
  type.value = 'node'
  node.value = n
  isShow.value = true
  void nextTick(() => positionAt(e.clientX + 10, e.clientY + 10))
}

function showCanvas(e: MouseEvent): void {
  type.value = 'svg'
  node.value = null
  isShow.value = true
  void nextTick(() => positionAt(e.clientX + 10, e.clientY + 10))
}

function hide(): void {
  isShow.value = false
  left.value = -9999
  top.value = -9999
  node.value = null
}

function onSvgMousedown(e: MouseEvent): void {
  // which===3 或 button===2 为右键。
  if (e.button !== 2 && (e as MouseEvent & { which?: number }).which !== 3) return
  mousedownX = e.clientX
  mousedownY = e.clientY
  isMousedown = true
}
function onNodeMousedown(): void {
  isNodeMousedown = true
}
function onMouseup(e: MouseEvent): void {
  if (!isMousedown) return
  if (isNodeMousedown) {
    isNodeMousedown = false
    return
  }
  isMousedown = false
  if (Math.abs(mousedownX - e.clientX) > 3 || Math.abs(mousedownY - e.clientY) > 3) {
    hide()
    return
  }
  showCanvas(e)
}

/** 节点/画布命令。disabled 时忽略。 */
function exec(key: string, disabled = false, ...args: unknown[]): void {
  if (disabled) return
  const mindMap = requireMindMap(ctx)
  switch (key) {
    case 'COPY_NODE':
      mindMap.renderer.copy()
      break
    case 'CUT_NODE':
      mindMap.renderer.cut()
      break
    case 'PASTE_NODE':
      mindMap.renderer.paste()
      break
    case 'RETURN_CENTER':
      mindMap.renderer.setRootNodeCenter()
      break
    case 'TOGGLE_ZEN_MODE':
      localConfig.isZenMode = !localConfig.isZenMode
      break
    case 'FIT_CANVAS':
      mindMap.view.fit()
      break
    case 'REMOVE_HYPERLINK':
      node.value?.setHyperlink('', '')
      break
    case 'REMOVE_NOTE':
      node.value?.setNote('')
      break
    case 'EXPORT_CUR_NODE_TO_PNG':
      if (node.value) {
        void mindMap.export('png', true, getTextFromHtml(node.value.getData('text')), false, node.value)
      }
      break
    case 'UNEXPAND_ALL': {
      const uid = node.value ? node.value.uid : ''
      bus.emit('execCommand', key, !uid, uid)
      break
    }
    case 'EXPAND_ALL':
      bus.emit('execCommand', key, node.value ? node.value.uid : '')
      break
    default:
      bus.emit('execCommand', key, ...args)
      break
  }
  hide()
}

/** 展开到第 N 级（画布菜单）。 */
function unexpandToLevel(level: number): void {
  bus.emit('execCommand', 'UNEXPAND_TO_LEVEL', level)
  hide()
}

/** 复制整图到剪贴板的五种格式。 */
async function copyToClipboard(format: string): Promise<void> {
  const mindMap = requireMindMap(ctx)
  hide()
  try {
    let str = ''
    if (format === 'smm' || format === 'json') str = JSON.stringify(mindMap.getData(true))
    else if (format === 'md') str = transformToMarkdown(mindMap.getData())
    else if (format === 'txt') str = transformToTxt(mindMap.getData())
    else if (format === 'png') {
      const png = await mindMap.export('png', false)
      const blob = await imgToDataUrl(png, true)
      if (navigator.clipboard?.write) {
        await navigator.clipboard.write([new ClipboardItem({ 'image/png': blob as Blob })])
      }
    }
    if (str && navigator.clipboard?.writeText) await navigator.clipboard.writeText(str)
    toast('已复制到剪贴板')
  } catch (error) {
    logger.error('mindmap', '复制到剪贴板失败', error)
    toast('复制失败')
  }
}

const offs: Array<() => void> = []
onMounted(() => {
  offs.push(bus.on('node_contextmenu', (e, n) => showNode(e as MouseEvent, n as MindMapNode)))
  offs.push(bus.on('node_click', hide))
  offs.push(bus.on('draw_click', hide))
  offs.push(bus.on('expand_btn_click', hide))
  offs.push(bus.on('svg_mousedown', (e) => onSvgMousedown(e as MouseEvent)))
  offs.push(bus.on('mouseup', (e) => onMouseup(e as MouseEvent)))
  offs.push(bus.on('translate', hide))
  offs.push(bus.on('node_mousedown', onNodeMousedown))
})
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <div
    v-show="isShow"
    ref="menuRef"
    class="mm-ctxmenu mm-panel"
    :class="{ 'sub-left': subLeft }"
    :style="{ left: `${left}px`, top: `${top}px` }"
  >
    <!-- 节点菜单 -->
    <template v-if="type === 'node'">
      <div class="mm-ctx-item" :class="{ disabled: insertDisabled }" @click="exec('INSERT_NODE', insertDisabled)">
        插入同级节点
      </div>
      <div class="mm-ctx-item" @click="exec('INSERT_CHILD_NODE')">插入子级节点</div>
      <div
        class="mm-ctx-item"
        :class="{ disabled: insertDisabled }"
        @click="exec('INSERT_PARENT_NODE', insertDisabled)"
      >
        插入父节点
      </div>
      <div
        class="mm-ctx-item"
        :class="{ disabled: insertDisabled }"
        @click="exec('ADD_GENERALIZATION', insertDisabled)"
      >
        插入概要
      </div>
      <div class="mm-ctx-sep" />
      <div class="mm-ctx-item" :class="{ disabled: upDisabled }" @click="exec('UP_NODE', upDisabled)">上移节点</div>
      <div class="mm-ctx-item" :class="{ disabled: downDisabled }" @click="exec('DOWN_NODE', downDisabled)">
        下移节点
      </div>
      <div class="mm-ctx-item" @click="exec('UNEXPAND_ALL')">收起所有下级节点</div>
      <div class="mm-ctx-item" @click="exec('EXPAND_ALL')">展开所有下级节点</div>
      <div class="mm-ctx-sep" />
      <div class="mm-ctx-item danger" @click="exec('REMOVE_NODE')">删除节点</div>
      <div class="mm-ctx-item danger" @click="exec('REMOVE_CURRENT_NODE')">仅删除当前节点</div>
      <div class="mm-ctx-item" @click="exec('COPY_NODE')">复制节点</div>
      <div class="mm-ctx-item" @click="exec('CUT_NODE')">剪切节点</div>
      <div class="mm-ctx-item" @click="exec('PASTE_NODE')">粘贴节点</div>
      <div v-if="hasHyperlink" class="mm-ctx-item" @click="exec('REMOVE_HYPERLINK')">移除超链接</div>
      <div v-if="hasNote" class="mm-ctx-item" @click="exec('REMOVE_NOTE')">移除备注</div>
      <div class="mm-ctx-item" @click="exec('REMOVE_CUSTOM_STYLES')">一键去除自定义样式</div>
      <div class="mm-ctx-item" @click="exec('EXPORT_CUR_NODE_TO_PNG')">导出该节点为图片</div>
    </template>

    <!-- 画布菜单 -->
    <template v-else>
      <div class="mm-ctx-item" @click="exec('RETURN_CENTER')">回到根节点</div>
      <div class="mm-ctx-item" @click="exec('EXPAND_ALL')">展开所有</div>
      <div class="mm-ctx-item" @click="exec('UNEXPAND_ALL')">收起所有</div>
      <div class="mm-ctx-item has-sub">
        展开到
        <div class="mm-ctx-sub" :class="{ left: subLeft }">
          <div
            v-for="(label, index) in expandLevels"
            :key="index"
            class="mm-ctx-item"
            @click="unexpandToLevel(index + 1)"
          >
            {{ label }}
          </div>
        </div>
      </div>
      <div class="mm-ctx-sep" />
      <div class="mm-ctx-item" @click="exec('RESET_LAYOUT')">一键整理布局</div>
      <div class="mm-ctx-item" @click="exec('FIT_CANVAS')">适应画布</div>
      <div class="mm-ctx-item" @click="exec('TOGGLE_ZEN_MODE')">{{ ui.isZenMode ? '退出禅模式' : '禅模式' }}</div>
      <div class="mm-ctx-item" @click="exec('REMOVE_ALL_NODE_CUSTOM_STYLES')">一键去除所有节点自定义样式</div>
      <div class="mm-ctx-item has-sub">
        复制到剪贴板
        <div class="mm-ctx-sub" :class="{ left: subLeft }">
          <div v-for="item in copyList" :key="item.value" class="mm-ctx-item" @click="copyToClipboard(item.value)">
            {{ item.name }}
          </div>
        </div>
      </div>
    </template>
  </div>
</template>
