import { invoke } from '@tauri-apps/api/core'
import { createApp, h } from 'vue'
import '@/styles'

createApp({
  setup() {
    const showPanel = () => void invoke('panel_show')
    return () => h('div', { class: 'hotzone', onMouseenter: showPanel, onClick: showPanel, title: '呼出 Inkling' })
  },
}).mount('#app')
