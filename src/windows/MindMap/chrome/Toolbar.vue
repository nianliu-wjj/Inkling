<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import type { MindMapNode } from 'simple-mind-map'
import { useMindMap } from '../core/useMindMap'

/**
 * 顶部工具栏：撤销/重做/格式刷 + 节点操作（插入、删除、图片、图标、超链接、备注、
 * 标签、概要、关联线、公式、外框）+ 文件（导入、导出）。
 *
 * 与参考端 ToolbarNodeBtnList 一致：按钮的可用性随当前激活节点变化——
 * 无选中节点时节点类操作禁用；根节点不能插入同级/父节点/概要；概要节点不能再加概要。
 * 状态通过 bus 订阅库事件维护，命令通过 execCommand / 具名 bus 事件下发。
 */
const { bus, ui } = useMindMap()

/** 撤销/重做可用性（back_forward 事件回传当前指针与历史长度）。 */
const backEnd = ref(true)
const forwardEnd = ref(true)
/** 是否处于格式刷取样态。 */
const inPainter = ref(false)

/** 是否有激活节点。 */
const hasActive = () => ui.activeNodes.length > 0
/** 激活集合里是否包含根节点 / 概要节点（决定部分按钮禁用）。 */
const hasRoot = () => ui.activeNodes.some((n: MindMapNode) => n.isRoot)
const hasGeneralization = () => ui.activeNodes.some((n: MindMapNode) => n.isGeneralization)

const onBackForward = (index: number, len: number): void => {
  backEnd.value = index <= 0
  forwardEnd.value = index >= len - 1
}
const onPainterStart = (): void => {
  inPainter.value = true
}
const onPainterEnd = (): void => {
  inPainter.value = false
}

const offs: Array<() => void> = []
onMounted(() => {
  offs.push(bus.on('back_forward', (i, l) => onBackForward(i as number, l as number)))
  offs.push(bus.on('painter_start', onPainterStart))
  offs.push(bus.on('painter_end', onPainterEnd))
})
onBeforeUnmount(() => offs.forEach((off) => off()))

/** 下发库命令。 */
function exec(cmd: string, ...args: unknown[]): void {
  bus.emit('execCommand', cmd, ...args)
}

/** 打开某个侧栏（图标 / 公式借用侧栏承载）。 */
function openSidebar(name: typeof ui.activeSidebar): void {
  ui.activeSidebar = ui.activeSidebar === name ? '' : name
}

interface Btn {
  key: string
  icon: string
  label: string
  /** 是否禁用。 */
  disabled?: () => boolean
  run: () => void
}

/** 文件与历史组。 */
const fileButtons: Btn[] = [
  { key: 'import', icon: 'icondaoru', label: '导入', run: () => bus.emit('showImport') },
  { key: 'export', icon: 'icondaochu', label: '导出', run: () => bus.emit('showExport') },
  { key: 'back', icon: 'iconhoutui-shi', label: '回退', disabled: () => backEnd.value, run: () => exec('BACK') },
  { key: 'forward', icon: 'iconqianjin1', label: '前进', disabled: () => forwardEnd.value, run: () => exec('FORWARD') },
  {
    key: 'painter',
    icon: 'icongeshihua',
    label: '格式刷',
    disabled: () => !hasActive(),
    run: () => bus.emit('startPainter'),
  },
]

/** 节点操作组。 */
const nodeButtons: Btn[] = [
  {
    key: 'sibling',
    icon: 'iconjiedian',
    label: '同级节点',
    disabled: () => !hasActive() || hasRoot() || hasGeneralization(),
    run: () => exec('INSERT_NODE'),
  },
  {
    key: 'child',
    icon: 'icontianjiazijiedian',
    label: '子节点',
    disabled: () => !hasActive() || hasGeneralization(),
    run: () => exec('INSERT_CHILD_NODE'),
  },
  {
    key: 'delete',
    icon: 'iconshanchu',
    label: '删除节点',
    disabled: () => !hasActive(),
    run: () => exec('REMOVE_NODE'),
  },
  {
    key: 'image',
    icon: 'iconimage',
    label: '图片',
    disabled: () => !hasActive(),
    run: () => bus.emit('showNodeImage'),
  },
  {
    key: 'icon',
    icon: 'iconxiaolian',
    label: '图标',
    disabled: () => !hasActive(),
    run: () => openSidebar('nodeIconSidebar'),
  },
  {
    key: 'link',
    icon: 'iconchaolianjie',
    label: '超链接',
    disabled: () => !hasActive(),
    run: () => bus.emit('showNodeLink'),
  },
  {
    key: 'linkNode',
    icon: 'iconlianjie',
    label: '链接节点',
    disabled: () => !hasActive(),
    run: () => bus.emit('showNodeLinkToNode', ui.activeNodes[0]),
  },
  {
    key: 'attachment',
    icon: 'iconfujian',
    label: '附件',
    disabled: () => !hasActive(),
    run: () => bus.emit('selectAttachment', ui.activeNodes),
  },
  {
    key: 'note',
    icon: 'iconbiaoqian',
    label: '备注',
    disabled: () => !hasActive(),
    run: () => bus.emit('showNodeNote'),
  },
  { key: 'tag', icon: 'iconbiaoqian', label: '标签', disabled: () => !hasActive(), run: () => bus.emit('showNodeTag') },
  {
    key: 'summary',
    icon: 'icongaikuozonglan',
    label: '概要',
    disabled: () => !hasActive() || hasRoot() || hasGeneralization(),
    run: () => exec('ADD_GENERALIZATION'),
  },
  {
    key: 'assocLine',
    icon: 'iconlianjiexian',
    label: '关联线',
    disabled: () => !hasActive(),
    run: () => bus.emit('createAssociativeLine'),
  },
  {
    key: 'formula',
    icon: 'icongongshi',
    label: '公式',
    disabled: () => !hasActive(),
    run: () => openSidebar('formulaSidebar'),
  },
  {
    key: 'outerFrame',
    icon: 'iconwaikuang',
    label: '外框',
    disabled: () => !hasActive(),
    run: () => exec('ADD_OUTER_FRAME'),
  },
]
</script>

<template>
  <div class="mm-toolbar mm-panel">
    <div class="mm-toolbar-group">
      <button
        v-for="btn in fileButtons"
        :key="btn.key"
        type="button"
        class="mm-tool-btn"
        :class="{ active: btn.key === 'painter' && inPainter }"
        :disabled="btn.disabled?.()"
        :title="btn.label"
        @click="btn.run"
      >
        <i class="iconfont" :class="btn.icon" />
        <span class="mm-tool-label">{{ btn.label }}</span>
      </button>
    </div>
    <div class="mm-toolbar-sep" />
    <div class="mm-toolbar-group">
      <button
        v-for="btn in nodeButtons"
        :key="btn.key"
        type="button"
        class="mm-tool-btn"
        :disabled="btn.disabled?.()"
        :title="btn.label"
        @click="btn.run"
      >
        <i class="iconfont" :class="btn.icon" />
        <span class="mm-tool-label">{{ btn.label }}</span>
      </button>
    </div>
  </div>
</template>
