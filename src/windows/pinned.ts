/** 桌面置顶浮窗入口。每个置顶项对应一个独立窗口。 */
import { createApp } from 'vue'
import PinnedApp from './Pinned/PinnedApp.vue'
import '@/styles'

createApp(PinnedApp).mount('#app')
