<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { formulaList } from '../constants/formulas'
import { requireMindMap, useMindMap } from '../core/useMindMap'
import MmModal from './MmModal.vue'

/**
 * 节点公式弹窗（原型 `docs/app.js:1964-2004` 对通用弹窗的用法，spec D42）。
 *
 * 内容从 `sidebars/FormulaSidebar.vue` 迁入，只换渲染宿主（右侧抽屉 → 工具栏按钮弹出的模态）：
 * LaTeX 输入框 + 常用公式表都是我们的真实现（spec D47），表格改挂生成层的 `.mm-formula-table`
 * ——整行可点即填入输入框，对应原型 `docs/app.js:2000-2003` 的行点击；
 * 旧侧栏那行是 `<button :disabled>`，天生可 Tab，故行上补 `role="button"` + tabindex + Enter/空格，
 * 把表格形态下的键盘可达补回来。
 * 非富文本模式下整体禁用并给提示（公式渲染依赖节点富文本），与侧栏时期一致。
 *
 * @author nianliu-jj
 * @since 2026-09-14
 */
const ctx = useMindMap()
const { ui, localConfig } = ctx
const { toast } = useToast()

const visible = ref(false)
const latex = ref('')
const richText = computed(() => localConfig.openNodeRichText)

/** 打开时回显当前节点的第一个公式（若有）。 */
function syncFromNode(): void {
  const node = ui.activeNodes[0]
  if (!node) return
  const formulas = (node.getData('formula') as string[] | undefined) ?? []
  if (formulas.length) latex.value = formulas[0]
}

/** 打开弹窗：无激活节点时提示并中止（工具栏按钮已禁用，这里是兜底）。 */
function open(): void {
  if (!ui.activeNodes.length) {
    toast('请先选择一个节点')
    return
  }
  syncFromNode()
  visible.value = true
}

/** 点选常用公式：只填入输入框，是否插入由「确定」决定（与原型同）。 */
function pick(formula: string): void {
  // 非富文本模式下点选无效：旧侧栏靠 `button:disabled` 挡住，表格行没有 disabled 属性，故在此拦。
  if (!richText.value) return
  latex.value = formula
}

/**
 * 确定：把输入框里的 LaTeX 插到当前节点。
 * 成功后关窗——原型是「回调执行完即 `closeMmModal()`」（`docs/app.js:1849-1852`）。
 */
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
  logger.info('mindmap', `节点公式：插入 ${text}`)
  latex.value = ''
  visible.value = false
}

const offs: Array<() => void> = []
onMounted(() => {
  offs.push(ctx.bus.on('showNodeFormula', open))
  // 回显随激活节点变化（与侧栏时期一致）
  offs.push(ctx.bus.on('node_active', syncFromNode))
})
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <MmModal :visible="visible" title="插入公式 (LaTeX)" @close="visible = false">
    <p v-if="!richText" class="mm-dialog-hint">非富文本模式下不支持插入公式，请在「设置」中开启节点富文本编辑。</p>
    <textarea
      v-model="latex"
      class="mm-textarea mm-formula-input"
      :disabled="!richText"
      placeholder="请输入 LaTeX 语法，例如 \frac{1}{2}"
    />
    <div class="mm-setting-group">常用公式（点击填入）</div>
    <!-- 生成层 `.mm-formula-table` 的 `tr:hover` 就是「整行可点」的视觉反馈。
         行上的 `role="button"` 同时把它接进 base.css 全局焦点环那条
         `[role='button']:focus-visible` 规则，故无需新增任何样式。
         键盘可达是相对旧侧栏（`<button :disabled>` 可 Tab、有 disabled 语义）的补回：
         Enter / 空格填入，非富文本时 tabindex=-1 且 aria-disabled，不再响应键盘（`pick` 里也拦一道）。

         `role="presentation"`（Task 4 审查补）：`tr` 上的 `role="button"` 覆盖了它隐含的 `row`，
         而 `<table>` 的 required-owned-elements 要求子项是 rowgroup/row ⇒ 结构不成立。让表格
         整体让掉隐式的 table/row/cell 角色后，这一块就是「一串按钮」，结构合法、零 CSS 代价。
         **不把表格语义改回来**是权衡后的取舍：这份「表格」只剩单列、没有 `<th>` / `<caption>` /
         `<thead>`，行语义的信息量≈0（读屏念出来与列表无异）；而按钮语义换来的是真收益——
         Tab 可达、Enter / 空格可点、`aria-disabled` 能表达禁用、能吃到全局焦点环。 -->
    <table class="mm-formula-table" :class="{ disabled: !richText }" role="presentation">
      <tbody>
        <tr
          v-for="item in formulaList"
          :key="item"
          :title="item"
          role="button"
          :tabindex="richText ? 0 : -1"
          :aria-disabled="richText ? undefined : 'true'"
          @click="pick(item)"
          @keydown.enter.prevent="pick(item)"
          @keydown.space.prevent="pick(item)"
        >
          <td class="mm-formula-tex">{{ item }}</td>
        </tr>
      </tbody>
    </table>
    <template #footer>
      <button type="button" class="btn ghost" @click="visible = false">取消</button>
      <button type="button" class="btn primary" :disabled="!richText" @click="insert">确定</button>
    </template>
  </MmModal>
</template>
