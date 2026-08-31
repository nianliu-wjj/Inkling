import { invoke } from '@tauri-apps/api/core'
import { createApp, ref } from 'vue'
import '@/styles/windows.css'

const Panel = {
  setup() {
    const mode = ref<'note' | 'todo'>('note')
    const content = ref('')
    const busy = ref(false)
    const message = ref('')
    const close = () => void invoke('panel_hide')
    const save = async () => {
      if (!content.value.trim()) return
      busy.value = true
      try {
        if (mode.value === 'note') {
          await invoke('note_save', { input: { content: content.value, tags: [], draft: false } })
        } else {
          await invoke('todo_save', {
            input: {
              content: content.value,
              dueAt: new Date(Date.now() + 60 * 60 * 1000).toISOString(),
              remindAt: null,
              repeatRule: null,
              priority: 'medium',
              remark: '',
              tags: [],
              parentId: null,
            },
          })
        }
        content.value = ''
        message.value = mode.value === 'note' ? '念头已归档' : '待办已创建'
      } catch (error) {
        message.value = String(error)
      } finally {
        busy.value = false
      }
    }
    const capture = async () => {
      busy.value = true
      try {
        const result = await invoke('clipboard_capture')
        message.value = result ? '剪贴板已捕获' : '剪贴板为空'
      } catch (error) {
        message.value = String(error)
      } finally {
        busy.value = false
      }
    }
    return { mode, content, busy, message, close, save, capture }
  },
  template: `
    <section class="quick-panel" @keydown.esc="close">
      <header><strong>✒ Inkling</strong><button class="window-button" @click="close">×</button></header>
      <div class="panel-tabs"><button :class="{active: mode === 'note'}" @click="mode = 'note'">念头</button><button :class="{active: mode === 'todo'}" @click="mode = 'todo'">待办</button></div>
      <textarea v-model="content" autofocus :placeholder="mode === 'note' ? '写下此刻的念头…' : '输入待办内容…'" @keydown.ctrl.enter.prevent="save"></textarea>
      <footer><span>{{ message }}</span><button class="secondary" @click="capture" :disabled="busy">捕获剪贴板</button><button class="primary" @click="save" :disabled="busy || !content.trim()">保存</button></footer>
    </section>
  `,
}
createApp(Panel).mount('#app')
