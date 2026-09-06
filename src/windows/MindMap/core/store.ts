import { reactive } from 'vue'
import type { MindMapNode } from 'simple-mind-map'

/** 侧栏标识，与参考端 `activeSidebar` 取值一致。 */
export type SidebarName =
  | 'nodeStyle'
  | 'baseStyle'
  | 'theme'
  | 'structure'
  | 'outline'
  | 'setting'
  | 'shortcutKey'
  | 'nodeIconSidebar'
  | 'formulaSidebar'
  | 'nodeNoteSidebar'
  | ''

/**
 * 导图窗口内跨组件共享的 UI 状态（替代参考端 vuex 的非持久化部分）。
 * 持久化偏好见 localConfig.ts。
 */
export interface MindMapUiState {
  activeSidebar: SidebarName
  isReadonly: boolean
  isZenMode: boolean
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
    isOutlineEdit: false,
    isSourceCodeEdit: false,
    isDragOutlineTreeNode: false,
    extraTextOnExport: '',
    activeNodes: [],
  })
}
