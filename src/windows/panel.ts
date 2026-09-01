/**
 * 呼出面板窗口入口。
 *
 * 该窗口对启动速度敏感（需求「1 秒原则」），因此只加载面板自身依赖，
 * 不引入归档页、统计图表等重量级模块。
 */
import { createApp } from 'vue'
import PanelApp from './Panel/PanelApp.vue'
import '@/styles'

createApp(PanelApp).mount('#app')
