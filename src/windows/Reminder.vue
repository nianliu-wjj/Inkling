<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'
import { onMounted, ref } from 'vue'

type ReminderTodo = {
  id: string
  content: string
  due_at: string
}

const todo = ref<ReminderTodo | null>(null)
const todoId = getCurrentWindow().label.replace(/^reminder-/, '')

const load = async () => {
  const todos = await invoke<ReminderTodo[]>('todos_list')
  todo.value = todos.find((item) => item.id === todoId) || null
}

const close = () => void invoke('reminder_close', { todoId })

const complete = async () => {
  await invoke('todo_complete', { id: todoId, completed: true })
  close()
}

const snooze = async (minutes: number) => {
  await invoke('todo_snooze', { id: todoId, minutes })
  close()
}

const dismiss = async () => {
  await invoke('todo_dismiss_reminder', { id: todoId })
  close()
}

onMounted(() => {
  void load()
  void listen('inkling://reminder-fired', () => void load())
})
</script>

<template>
  <article class="reminder-card">
    <header>
      <strong>⏰ 待办提醒</strong>
      <button class="window-button" @click="close">×</button>
    </header>
    <div v-if="todo">
      <h3>{{ todo.content }}</h3>
      <p>计划完成：{{ new Date(todo.due_at).toLocaleString() }}</p>
      <div class="reminder-actions">
        <button class="secondary" @click="snooze(10)">10 分钟后</button>
        <button class="secondary" @click="dismiss">不再提醒</button>
        <button class="primary" @click="complete">完成</button>
      </div>
    </div>
    <div v-else class="muted">待办已删除或完成。</div>
  </article>
</template>
