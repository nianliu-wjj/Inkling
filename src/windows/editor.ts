/**
 * 独立编辑窗口入口。
 *
 * 铺满显示器工作区的透明置顶窗；待办编辑浮层在窗内锚定到触发卡片右侧（spec D23），
 * 面板（480px）与主窗口都借这个窗口摆浮层，不被各自的窗口边界裁切。
 */
import { createApp } from 'vue'
import EditorApp from './Editor/EditorApp.vue'
import '@/styles'

createApp(EditorApp).mount('#app')
