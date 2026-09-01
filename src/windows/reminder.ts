/** 待办提醒卡片入口。到期时由后端调度器创建对应窗口。 */
import { createApp } from 'vue'
import ReminderApp from './Reminder/ReminderApp.vue'
import '@/styles'

createApp(ReminderApp).mount('#app')
