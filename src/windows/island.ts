/**
 * 灵动岛窗口入口。
 *
 * 主屏顶部居中的胶囊窗口（Rust 侧创建于感应区正下方），透明、置顶、不进任务栏。
 * 内容由 `@/island-plugins` 注册表提供并轮播；悬停展开详情、左键点击唤出面板到待办页。
 * 悬停 / 点击事件统一由后端光标轮询推送（穿透模式下窗口自身收不到鼠标事件）。
 */
import { createApp } from 'vue'
import IslandApp from './Island/IslandApp.vue'
import '@/styles'
import '@/styles/island.css'

createApp(IslandApp).mount('#app')
