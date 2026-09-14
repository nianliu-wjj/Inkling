<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import type { MindMapNode } from 'simple-mind-map'
import { useMindMap } from '../core/useMindMap'
import MmFilenameIsland from './MmFilenameIsland.vue'

/**
 * 顶栏三岛（原型 `docs/index.html:311-360`）：左岛节点/内容工具组、中间文件名岛、右岛文件与存储。
 *
 * 与参考端 ToolbarNodeBtnList 一致：按钮的可用性随当前激活节点变化——
 * 无选中节点时节点类操作禁用；根节点不能插入同级/父节点/概要；概要节点不能再加概要。
 * 状态通过 bus 订阅库事件维护，命令通过 execCommand / 具名 bus 事件下发。
 *
 * 相对原型的取舍：
 * - 保留「链接节点」「附件」两枚（我们的真能力，原型没有，spec §3 差异表 #2）；
 * - 不放 AI 按钮（spec §1）；
 * - 图标继续用 iconfont 矢量字形（原型是 emoji）：三岛形态照原型，字形技术沿用现状，
 *   换成 emoji 会让整个窗口的图标语言与其余部分割裂，属于本阶段范围外的纯外观变动。
 *
 * 本组件只负责展示与上抛事件：改名 / 保存 / 新建 / 关闭的落库与状态变更都在 MindMapApp。
 */
defineProps<{ mapName: string }>()
const emit = defineEmits<{ rename: [name: string]; save: []; close: []; 'new-map': [] }>()

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

interface Btn {
  key: string
  icon: string
  label: string
  /** 是否禁用。 */
  disabled?: () => boolean
  /** 危险操作（删除节点）：hover 变红，走生成层 `.mm-tool-btn.danger`。 */
  danger?: boolean
  run: () => void
}

/**
 * 历史与格式刷组（左岛开头三项）。
 * 原 `fileButtons` 里的导入 / 导出已按原型迁到右岛，这里只剩画布状态类操作。
 */
const historyButtons: Btn[] = [
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

/** 节点操作组（顺序照原型，并在「超链接」后插入我们多出的链接节点 / 附件）。 */
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
    danger: true,
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
    // 图标 / 公式已按 D42 从右侧抽屉改为模态，与其它节点类操作一样走 bus 事件（取渲染宿主在 popups/）
    run: () => bus.emit('showNodeIcon'),
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
    run: () => bus.emit('showNodeFormula'),
  },
  {
    key: 'outerFrame',
    icon: 'iconwaikuang',
    label: '外框',
    disabled: () => !hasActive(),
    run: () => exec('ADD_OUTER_FRAME'),
  },
]

/** 左岛按钮：历史/格式刷在前，节点操作在后（原型顺序）。 */
const leftButtons: Btn[] = [...historyButtons, ...nodeButtons]
</script>

<template>
  <div class="mm-topbar">
    <!-- 左岛：节点与内容编辑工具组（原型 docs/index.html:313-330，去掉 AI） -->
    <div class="mm-island mm-island-left">
      <button
        v-for="btn in leftButtons"
        :key="btn.key"
        type="button"
        class="mm-tool-btn"
        :class="{ danger: btn.danger, active: btn.key === 'painter' && inPainter }"
        :title="btn.label"
        :disabled="btn.disabled?.()"
        @click="btn.run"
      >
        <i class="iconfont" :class="btn.icon" />
        <span class="mm-txt">{{ btn.label }}</span>
      </button>
    </div>

    <!-- 中间：文件名岛（原型 docs/index.html:333-346） -->
    <MmFilenameIsland :name="mapName" @rename="emit('rename', $event)" />

    <!-- 右岛：文件与存储（原型 docs/index.html:349-359） -->
    <div class="mm-island mm-island-right">
      <button type="button" class="mm-tool-btn" title="保存到 Inkling 笔记（Ctrl+S）" @click="emit('save')">
        <i class="iconfont iconlingcunwei" /><span class="mm-txt">保存</span>
      </button>
      <button type="button" class="mm-tool-btn" title="新建导图（清空当前内容）" @click="emit('new-map')">
        <i class="iconfont iconxinjian" /><span class="mm-txt">新建</span>
      </button>
      <button type="button" class="mm-tool-btn" title="导入 JSON 导图" @click="bus.emit('showImport')">
        <i class="iconfont icondaoru" /><span class="mm-txt">导入</span>
      </button>
      <button
        type="button"
        class="mm-tool-btn"
        title="导出思维导图（选择格式、存储路径与文件名）"
        @click="bus.emit('showExport')"
      >
        <i class="iconfont icondaochu" /><span class="mm-txt">导出</span>
      </button>
      <button type="button" class="mm-tool-btn" title="关闭窗口" @click="emit('close')">
        <i class="iconfont iconguanbi" /><span class="mm-txt">关闭</span>
      </button>
    </div>
  </div>
</template>
