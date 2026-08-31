import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { createApp, ref, onMounted } from 'vue'
import '@/styles/windows.css'

const Pinned = {
  setup() {
    const kind = ref('')
    const item = ref<any>(null)
    const editing = ref(false)
    const label = getCurrentWindow().label
    const close = () => void invoke('pin_close', { label: label })
    const load = async () => {
      const match = label.match(/^pinned-(note|todo|clip)-(.+)$/)
      if (!match) return
      kind.value = match[1]
      const list = await invoke<any[]>(
        kind.value === 'note' ? 'notes_list' : kind.value === 'todo' ? 'todos_list' : 'clipboard_list',
      )
      item.value = list.find((entry) => entry.id === match[2]) || null
    }
    const paste = () => item.value && kind.value === 'clip' && void invoke('clipboard_write', { id: item.value.id })
    onMounted(() => void load())
    return { kind, item, editing, close, paste }
  },
  template: `
    <article class="floating-card" :class="kind">
      <header><span>{{ kind === 'note' ? '笔记' : kind === 'todo' ? '待办' : '剪贴板' }}</span><button class="window-button" @click="close">×</button></header>
      <div v-if="item" class="floating-content">{{ item.content || item.preview }}</div><div v-else class="floating-content muted">内容已不存在</div>
      <footer><button v-if="kind === 'clip'" class="secondary" @click="paste">复制</button><span class="muted">桌面置顶</span></footer>
    </article>
  `,
}
createApp(Pinned).mount('#app')
