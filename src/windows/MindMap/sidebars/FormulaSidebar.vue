<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useToast } from '@/composables/useToast'
import { formulaList } from '../constants/formulas'
import { requireMindMap, useMindMap } from '../core/useMindMap'
import SidebarShell from './SidebarShell.vue'

/**
 * 公式侧栏（移植参考端 FormulaSidebar.vue）。
 *
 * LaTeX 输入 + 常用公式点选；完成 → `INSERT_FORMULA`。公式渲染依赖富文本模式，
 * 非富文本时禁用并提示。
 */
const ctx = useMindMap()
const { ui, localConfig } = ctx
const { toast } = useToast()

const latex = ref('')
const richText = computed(() => localConfig.openNodeRichText)

function insert(): void {
  const text = latex.value.trim()
  if (!text) {
    toast('请输入 LaTeX 公式')
    return
  }
  if (!ui.activeNodes.length) {
    toast('请先选择一个节点')
    return
  }
  requireMindMap(ctx).execCommand('INSERT_FORMULA', text)
  latex.value = ''
}

function pick(formula: string): void {
  latex.value = formula
}

/** 侧栏打开时回显当前节点的第一个公式（若有）。 */
function onActive(): void {
  const node = ui.activeNodes[0]
  if (!node) return
  const formulas = (node.getData('formula') as string[] | undefined) ?? []
  if (formulas.length) latex.value = formulas[0]
}

const offs: Array<() => void> = []
onMounted(() => offs.push(ctx.bus.on('node_active', onActive)))
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <SidebarShell name="formulaSidebar" title="公式">
    <p v-if="!richText" class="mm-dialog-hint">非富文本模式下不支持插入公式，请在「设置」中开启节点富文本编辑。</p>
    <textarea
      v-model="latex"
      class="mm-dialog-textarea mm-formula-input"
      :disabled="!richText"
      placeholder="请输入 LaTeX 语法，例如 \frac{1}{2}"
    />
    <button type="button" class="btn primary" :disabled="!richText" @click="insert">完成</button>
    <div class="mm-setting-group">常用公式</div>
    <div class="mm-formula-list">
      <button
        v-for="item in formulaList"
        :key="item"
        type="button"
        class="mm-formula-item"
        :disabled="!richText"
        :title="item"
        @click="pick(item)"
      >
        {{ item }}
      </button>
    </div>
  </SidebarShell>
</template>
