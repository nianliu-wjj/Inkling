/**
 * 启动器搜索窗口入口。
 *
 * 全局快捷键（默认 Alt+Space）呼出的键盘驱动一次性查询窗口：光标所在屏居中偏上、560×420。
 * 输入即搜（后端索引 + 笔记 / 待办 / 浏览器历史），↑/↓ 选择、Enter 执行、Tab 切动作、Alt+1..9 直达、Ctrl+Enter 管理员、Esc/失焦隐藏。
 */
import { createApp } from 'vue'
import LauncherApp from './Launcher/LauncherApp.vue'
import '@/styles'

createApp(LauncherApp).mount('#app')
