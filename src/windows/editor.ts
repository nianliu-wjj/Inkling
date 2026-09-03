/**
 * 独立编辑窗口入口。
 *
 * 面板窗口只有 480px 宽、高度随内容伸缩（≤600px），编辑弹窗放在面板内必然被
 * 窗口边界裁切。该窗口铺满显示器工作区、背景为半透明压暗遮罩、对话框居中，
 * 因此对话框尺寸完全不受面板约束（参考原型 doc/index.html 的模态设计）。
 */
import { createApp } from 'vue'
import EditorApp from './Editor/EditorApp.vue'
import '@/styles'

createApp(EditorApp).mount('#app')
