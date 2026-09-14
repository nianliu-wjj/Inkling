import { reactive } from 'vue'
import type { MindMapNode } from 'simple-mind-map'

/**
 * 侧栏标识，与参考端 `activeSidebar` 取值一致。
 *
 * 不含图标 / 公式：它们在阶段五 Task 4 已按 spec D42 改为工具栏按钮弹出的模态
 * （`popups/NodeIconModal.vue` / `NodeFormulaModal.vue`），不再占用抽屉。
 */
export type SidebarName = 'nodeStyle' | 'baseStyle' | 'theme' | 'structure' | 'outline' | 'setting' | 'shortcutKey' | ''

/**
 * 导图窗口内跨组件共享的 UI 状态（替代参考端 vuex 的非持久化部分）。
 * 持久化偏好见 localConfig.ts。
 */
export interface MindMapUiState {
  activeSidebar: SidebarName
  isReadonly: boolean
  isZenMode: boolean
  /**
   * 小地图（Navigator）是否展开。
   *
   * 放在共享状态而不是底栏的局部 ref：底栏 `MmBottombar` 带 `v-if="!ui.isZenMode"`，
   * 进出禅模式会卸载重挂，局部 ref 随之复位成 false，而**订阅同一个开关的 Navigator 并不卸载**
   * ⇒ 小地图仍开着、按钮却显示成关闭态，出来后第一次点击"没反应"（要第二次才关）。
   * 状态提到这里后，按钮的 active 与点击都读同一个真值，重挂不再丢状态。
   */
  isMiniMapOpen: boolean
  isOutlineEdit: boolean
  isSourceCodeEdit: boolean
  isDragOutlineTreeNode: boolean
  /** 导出时底部附加文字（导出对话框写，createMindMap 的 addContentToFooter 读）。 */
  extraTextOnExport: string
  /** 最近一次 node_active 的激活节点列表。 */
  activeNodes: MindMapNode[]
}

export function createUiState(): MindMapUiState {
  return reactive<MindMapUiState>({
    activeSidebar: '',
    isReadonly: false,
    isZenMode: false,
    isMiniMapOpen: false,
    isOutlineEdit: false,
    isSourceCodeEdit: false,
    isDragOutlineTreeNode: false,
    extraTextOnExport: '',
    activeNodes: [],
  })
}
