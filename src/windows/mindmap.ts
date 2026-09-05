/**
 * 思维导图窗口入口。
 *
 * 独立顶层窗口，每个笔记一个（label 形如 `mindmap-<id>`，新建用 `mindmap-new`）：
 * 关闭主窗口不会连带关掉它，缩放与最大化也互不影响。
 * 保留系统标题栏——导图需要频繁缩放，且画布需要实心背景才看得清节点连线。
 */
import { createApp } from 'vue'
import MindMapApp from './MindMap/MindMapApp.vue'
import '@/styles'

createApp(MindMapApp).mount('#app')
