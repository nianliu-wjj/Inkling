/**
 * 启动器搜索窗口入口。
 *
 * 全局快捷键（默认 Alt+Space）呼出的键盘驱动一次性查询窗口：光标所在屏居中偏上。
 * 输入即搜（Rust 侧索引 + 匹配），↑/↓ 选择、Enter 启动、Ctrl+Enter 管理员、Tab 切动作、Esc/失焦隐藏。
 */
import { createApp } from 'vue'
import LauncherApp from './Launcher/LauncherApp.vue'
import '@/styles'
import '@/styles/launcher.css'

createApp(LauncherApp).mount('#app')
