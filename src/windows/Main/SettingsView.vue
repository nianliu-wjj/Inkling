<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useSettings } from '@/composables/useData'
import { useToast } from '@/composables/useToast'
import { useTheme } from '@/composables/useTheme'
import { themes } from '@/constants/themes'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { CollapsePolicy, RemarkStyle, Settings } from '@/typings/domain'

/**
 * 归档 · 偏好设置页。
 *
 * 需求 2.7：失焦收起策略 / 粘贴板保留天数 / 开机静默自启 / 全局快捷键（可重录）
 * / 备注展示样式 / 主题（30 套）。另加毛玻璃开关（本次设计新增）。
 *
 * 所有修改即时保存，并由后端广播 settings-changed 同步到其他窗口。
 */
const { settings, save } = useSettings()
const { applyTheme } = useTheme()
const { toast } = useToast()

const themeMenuOpen = ref(false)
/** 快捷键录制态：录制期间捕获所有按键。 */
const recording = ref(false)

const currentTheme = computed(() => themes.find((t) => t.key === settings.value.theme) ?? themes[0])

/** 统一的保存入口：局部覆盖后整体写回。 */
async function patch(partial: Partial<Settings>): Promise<void> {
  const next: Settings = { ...settings.value, ...partial }
  try {
    await save(next)
  } catch {
    toast('保存设置失败')
  }
}

async function pickTheme(key: string): Promise<void> {
  themeMenuOpen.value = false
  applyTheme(key)
  await patch({ theme: key })
}

/** 毛玻璃开关：调 IPC 应用效果 + 切换根属性让 CSS 降级为实色。 */
async function toggleAcrylic(enabled: boolean): Promise<void> {
  logger.info('settings', `切换毛玻璃 enabled=${enabled}`)
  try {
    await api.windows.setMainAcrylic(enabled)
  } catch (error) {
    logger.error('settings', '应用毛玻璃失败', error)
  }
  await patch({ main_acrylic: enabled })
}

// 根据设置同步 data-acrylic，供 base.css 的降级规则使用。
watch(
  () => settings.value.main_acrylic,
  (enabled) => {
    const root = document.documentElement
    if (enabled) root.removeAttribute('data-acrylic')
    else root.setAttribute('data-acrylic', 'off')
  },
  { immediate: true },
)

/** 录制全局快捷键：把按键组合规范化为 Tauri 接受的格式。 */
function onRecordKeydown(event: KeyboardEvent): void {
  if (!recording.value) return
  event.preventDefault()

  // 只按下修饰键时继续等待主键。
  const key = event.key
  if (['Control', 'Shift', 'Alt', 'Meta'].includes(key)) return

  const parts: string[] = []
  if (event.ctrlKey) parts.push('Ctrl')
  if (event.shiftKey) parts.push('Shift')
  if (event.altKey) parts.push('Alt')
  if (event.metaKey) parts.push('Super')
  parts.push(key === ' ' ? 'Space' : key.length === 1 ? key.toUpperCase() : key)

  const combo = parts.join('+')
  recording.value = false
  void rebind(combo)
}

async function rebind(combo: string): Promise<void> {
  logger.info('settings', `重新绑定全局快捷键 ${combo}`)
  try {
    const applied = await api.shortcut.rebind(combo)
    await patch({ shortcut: applied })
    toast(`快捷键已设为 ${applied}`)
  } catch (error) {
    logger.error('settings', '快捷键绑定失败', error)
    toast(`快捷键绑定失败：${String(error)}`)
  }
}

function startRecording(): void {
  recording.value = true
  toast('请按下新的快捷键组合')
  window.addEventListener('keydown', onRecordKeydown, { once: false })
}

/** 打开数据目录，便于用户查看落盘的笔记与图片。 */
async function openDataDir(): Promise<void> {
  try {
    const dir = await api.dataDir()
    await api.system.openPath(dir)
  } catch (error) {
    logger.error('settings', '打开数据目录失败', error)
    toast('打开数据目录失败')
  }
}
</script>

<template>
  <div class="archive-page">
    <div class="page-title">⚙️ 偏好设置</div>

    <div class="settings-body">
      <label class="setting-row">
        <span>失焦自动收起</span>
        <select
          :value="settings.collapse_policy"
          @change="patch({ collapse_policy: ($event.target as HTMLSelectElement).value as CollapsePolicy })"
        >
          <option value="immediate">立即收起</option>
          <option value="3s">延迟 3 秒收起</option>
          <option value="never">固定不收起</option>
        </select>
      </label>

      <label class="setting-row">
        <span>粘贴板保留天数</span>
        <input
          type="number"
          min="1"
          max="365"
          :value="settings.clipboard_retention_days"
          @change="patch({ clipboard_retention_days: Number(($event.target as HTMLInputElement).value) })"
        />
        天
      </label>

      <label class="setting-row">
        <span>开机静默自启动</span>
        <input
          type="checkbox"
          :checked="settings.start_on_boot"
          @change="patch({ start_on_boot: ($event.target as HTMLInputElement).checked })"
        />
      </label>

      <label class="setting-row">
        <span>全局快捷键</span>
        <kbd>{{ recording ? '按下组合键…' : settings.shortcut }}</kbd>
        <button type="button" class="btn tiny" :disabled="recording" @click="startRecording">重新录制</button>
      </label>

      <label class="setting-row">
        <span>备注展示样式</span>
        <select
          :value="settings.remark_style"
          @change="patch({ remark_style: ($event.target as HTMLSelectElement).value as RemarkStyle })"
        >
          <option value="mixed">混合模式（超 100 字用图标）</option>
          <option value="icon">图标徽章 + 悬浮</option>
          <option value="text">置灰文本行</option>
        </select>
      </label>

      <label class="setting-row">
        <span>窗口毛玻璃</span>
        <input
          type="checkbox"
          :checked="settings.main_acrylic"
          @change="toggleAcrylic(($event.target as HTMLInputElement).checked)"
        />
        <span class="clip-editor-hint">关闭后归档窗口使用不透明背景</span>
      </label>

      <div class="setting-row">
        <span>主题</span>
        <div class="theme-dd">
          <button type="button" class="theme-dd-trigger" @click="themeMenuOpen = !themeMenuOpen">
            <span class="theme-dots">
              <i v-for="dot in currentTheme.dots" :key="dot" :style="{ background: dot }" />
            </span>
            <span>{{ currentTheme.label }}</span>
          </button>
          <div v-if="themeMenuOpen" class="theme-dd-menu">
            <div
              v-for="option in themes"
              :key="option.key"
              class="theme-dd-opt"
              :class="{ active: option.key === settings.theme }"
              @click="pickTheme(option.key)"
            >
              <span class="dd-check">✓</span>
              <span class="theme-dots">
                <i v-for="dot in option.dots" :key="dot" :style="{ background: dot }" />
              </span>
              <span>{{ option.label }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="setting-row">
        <span>数据目录</span>
        <button type="button" class="btn tiny" @click="openDataDir">打开数据目录</button>
      </div>
    </div>
  </div>
</template>
