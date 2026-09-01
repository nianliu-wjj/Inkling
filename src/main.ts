/**
 * 归档主窗口入口。
 *
 * 该窗口承载笔记 / 粘贴板 / 待办 / 统计 / 偏好设置 / 日期详情六个视图
 * （v1.2 变更 #4：单窗口左右结构，不再弹独立窗口）。
 */
import { createApp } from 'vue'
import MainApp from './windows/Main/MainApp.vue'
import '@/styles'

createApp(MainApp).mount('#app')
