<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useSettings } from '@/composables/useData'
import { useToast } from '@/composables/useToast'
import { useGlass } from '@/composables/useGlass'
import { useTheme } from '@/composables/useTheme'
import { glassLevels } from '@/constants/glass'
import { builtinPlugins, resolvePlugins, serializePlugins } from '@/panel-plugins'
import { builtinIslandPlugins, resolveIslandPlugins, serializeIslandPlugins } from '@/island-plugins'
import { themes } from '@/constants/themes'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { CollapsePolicy, GlassLevel, PanelPosition, RemarkStyle, Settings } from '@/typings/domain'

/**
 * 归档 · 偏好设置页。
 *
 * 需求 2.7：失焦收起策略 / 粘贴板保留天数 / 开机静默自启 / 全局快捷键（可重录）
 * / 备注展示样式 / 主题（30 套）/ 玻璃质感（3 档，与配色正交）。
 * 另加毛玻璃开关与邮件提醒配置。
 *
 * 所有修改即时保存，并由后端广播 settings-changed 同步到其他窗口。
 */
const { settings, save } = useSettings()
const { applyTheme } = useTheme()
const { applyGlass } = useGlass()
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

/** 测试邮件发送中的状态，避免重复点击。 */
const mailTesting = ref(false)

/**
 * 发送一封测试邮件验证 SMTP 配置。
 *
 * 同步等待后端发送结果，失败时把具体错误（认证失败 / 连接超时等）原样提示，
 * 让用户能据此修正配置，而不是等到提醒到点才发现收不到信。
 */
async function sendTestMail(): Promise<void> {
  if (mailTesting.value) return
  mailTesting.value = true
  logger.info('settings', '发送测试邮件')
  try {
    await api.mail.test()
    toast('测试邮件已发送，请查收')
  } catch (error) {
    logger.error('settings', '发送测试邮件失败', error)
    toast(String(error))
  } finally {
    mailTesting.value = false
  }
}

/** 当前启用的插件 id 集合，用于勾选框的 checked 状态。 */
const enabledPluginIds = computed(
  () => new Set(resolvePlugins(settings.value.panel_plugins).map((plugin) => plugin.id)),
)

/**
 * 启用 / 禁用某个面板插件。
 *
 * 顺序始终沿用注册表的默认次序（勾选不改变排序），因此这里按 builtinPlugins
 * 过滤后重新序列化即可。全部取消勾选时不写入空串——空串会被 resolvePlugins
 * 兜回「全部启用」，与用户「一个都不要」的意图相反；因此至少保留一个。
 */
function togglePlugin(id: string, enabled: boolean): void {
  const next = new Set(enabledPluginIds.value)
  if (enabled) next.add(id)
  else next.delete(id)

  if (next.size === 0) {
    toast('至少需要启用一个面板插件')
    return
  }
  const ordered = builtinPlugins.filter((plugin) => next.has(plugin.id))
  logger.info('settings', `面板插件启用列表 = ${serializePlugins(ordered)}`)
  void patch({ panel_plugins: serializePlugins(ordered) })
}

/** 灵动岛尺寸等数值项的允许范围（与 Rust 侧 island_clamp 一致，前端先钳一遍避免来回抖动）。 */
const ISLAND_LIMITS = {
  width: { min: 200, max: 800 },
  height: { min: 28, max: 72 },
  opacity: { min: 0.3, max: 1 },
  cycle: { min: 2, max: 30 },
} as const

function clampNumber(value: number, range: { min: number; max: number }, fallback: number): number {
  if (!Number.isFinite(value)) return fallback
  return Math.min(range.max, Math.max(range.min, value))
}

/** 灵动岛数值设置：解析输入、钳制范围后保存。 */
function patchIslandNumber(
  key: 'island_width' | 'island_height' | 'island_opacity' | 'island_cycle_seconds',
  raw: string,
): void {
  const range =
    key === 'island_width'
      ? ISLAND_LIMITS.width
      : key === 'island_height'
        ? ISLAND_LIMITS.height
        : key === 'island_opacity'
          ? ISLAND_LIMITS.opacity
          : ISLAND_LIMITS.cycle
  const value = clampNumber(Number(raw), range, settings.value[key])
  logger.info('settings', `灵动岛 ${key} = ${value}`)
  void patch({ [key]: value } as Partial<Settings>)
}

/** 当前启用的灵动岛插件 id 集合。 */
const enabledIslandPluginIds = computed(
  () => new Set(resolveIslandPlugins(settings.value.island_plugins).map((plugin) => plugin.id)),
)

/** 启用 / 禁用某个灵动岛插件；与面板插件同理，至少保留一个。 */
function toggleIslandPlugin(id: string, enabled: boolean): void {
  const next = new Set(enabledIslandPluginIds.value)
  if (enabled) next.add(id)
  else next.delete(id)
  if (next.size === 0) {
    toast('至少需要启用一个灵动岛插件')
    return
  }
  const ordered = builtinIslandPlugins.filter((plugin) => next.has(plugin.id))
  logger.info('settings', `灵动岛插件启用列表 = ${serializeIslandPlugins(ordered)}`)
  void patch({ island_plugins: serializeIslandPlugins(ordered) })
}

/**
 * 切换玻璃质感。
 *
 * 与主题同构：先即时应用视觉再落盘，避免等一次 IPC 往返才看到变化。
 */
function pickGlass(level: GlassLevel): void {
  applyGlass(level)
  void patch({ glass_level: level })
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
        <span>面板唤出位置</span>
        <select
          :value="settings.panel_position"
          @change="patch({ panel_position: ($event.target as HTMLSelectElement).value as PanelPosition })"
        >
          <option value="top">顶部居中</option>
          <option value="bottom">底部居中</option>
          <option value="left">左侧居中</option>
          <option value="right">右侧居中</option>
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

      <label class="setting-row">
        <span>玻璃质感</span>
        <select
          :value="settings.glass_level"
          :title="glassLevels.find((item) => item.key === settings.glass_level)?.hint ?? ''"
          @change="pickGlass(($event.target as HTMLSelectElement).value as GlassLevel)"
        >
          <option v-for="option in glassLevels" :key="option.key" :value="option.key">
            {{ option.label }} · {{ option.hint }}
          </option>
        </select>
      </label>

      <div class="setting-row">
        <span>数据目录</span>
        <button type="button" class="btn tiny" @click="openDataDir">打开数据目录</button>
      </div>

      <div class="setting-section-title">面板插件</div>
      <p class="setting-hint">控制呼出面板显示哪些能力页。勾选顺序固定，序号即 <strong>⌃N</strong> 快捷键。</p>
      <label v-for="plugin in builtinPlugins" :key="plugin.id" class="setting-row">
        <span>{{ plugin.dot }} {{ plugin.label }}</span>
        <input
          type="checkbox"
          :checked="enabledPluginIds.has(plugin.id)"
          @change="togglePlugin(plugin.id, ($event.target as HTMLInputElement).checked)"
        />
      </label>

      <div class="setting-section-title">灵动岛</div>
      <p class="setting-hint">
        主屏顶部居中的胶囊，轮播当日待办；悬停查看详情，左键点击唤出面板到待办页。
        开启<strong>鼠标穿透</strong>后点击会直接穿到桌面，悬停与点击改由后端光标探测。
      </p>
      <label class="setting-row">
        <span>显示灵动岛</span>
        <input
          type="checkbox"
          :checked="settings.island_enabled"
          @change="patch({ island_enabled: ($event.target as HTMLInputElement).checked })"
        />
      </label>
      <label class="setting-row">
        <span>宽度（{{ ISLAND_LIMITS.width.min }}–{{ ISLAND_LIMITS.width.max }}）</span>
        <input
          type="number"
          :min="ISLAND_LIMITS.width.min"
          :max="ISLAND_LIMITS.width.max"
          :value="settings.island_width"
          @change="patchIslandNumber('island_width', ($event.target as HTMLInputElement).value)"
        />
      </label>
      <label class="setting-row">
        <span>高度（{{ ISLAND_LIMITS.height.min }}–{{ ISLAND_LIMITS.height.max }}）</span>
        <input
          type="number"
          :min="ISLAND_LIMITS.height.min"
          :max="ISLAND_LIMITS.height.max"
          :value="settings.island_height"
          @change="patchIslandNumber('island_height', ($event.target as HTMLInputElement).value)"
        />
      </label>
      <label class="setting-row">
        <span>背景不透明度 {{ Math.round(settings.island_opacity * 100) }}%</span>
        <input
          type="range"
          :min="ISLAND_LIMITS.opacity.min"
          :max="ISLAND_LIMITS.opacity.max"
          step="0.05"
          :value="settings.island_opacity"
          @change="patchIslandNumber('island_opacity', ($event.target as HTMLInputElement).value)"
        />
      </label>
      <label class="setting-row">
        <span>鼠标穿透（零干扰）</span>
        <input
          type="checkbox"
          :checked="settings.island_click_through"
          @change="patch({ island_click_through: ($event.target as HTMLInputElement).checked })"
        />
      </label>
      <label class="setting-row">
        <span>轮播间隔（秒，{{ ISLAND_LIMITS.cycle.min }}–{{ ISLAND_LIMITS.cycle.max }}）</span>
        <input
          type="number"
          :min="ISLAND_LIMITS.cycle.min"
          :max="ISLAND_LIMITS.cycle.max"
          :value="settings.island_cycle_seconds"
          @change="patchIslandNumber('island_cycle_seconds', ($event.target as HTMLInputElement).value)"
        />
      </label>
      <label v-for="plugin in builtinIslandPlugins" :key="plugin.id" class="setting-row">
        <span>🧩 {{ plugin.label }}</span>
        <input
          type="checkbox"
          :checked="enabledIslandPluginIds.has(plugin.id)"
          @change="toggleIslandPlugin(plugin.id, ($event.target as HTMLInputElement).checked)"
        />
      </label>

      <div class="setting-section-title">邮件提醒</div>
      <p class="setting-hint">请填写邮箱的<strong>应用专用密码</strong>而非主账号密码。配置保存在本地数据库中。</p>
      <label class="setting-row">
        <span>SMTP 服务器</span>
        <input
          :value="settings.smtp_host"
          placeholder="smtp.qq.com"
          @change="patch({ smtp_host: ($event.target as HTMLInputElement).value.trim() })"
        />
      </label>
      <label class="setting-row">
        <span>端口</span>
        <input
          :value="settings.smtp_port"
          type="number"
          min="1"
          max="65535"
          @change="patch({ smtp_port: Number(($event.target as HTMLInputElement).value) })"
        />
      </label>
      <label class="setting-row">
        <span>启用 TLS</span>
        <input
          :checked="settings.smtp_tls"
          type="checkbox"
          @change="patch({ smtp_tls: ($event.target as HTMLInputElement).checked })"
        />
      </label>
      <label class="setting-row">
        <span>账号</span>
        <input
          :value="settings.smtp_username"
          placeholder="you@example.com"
          @change="patch({ smtp_username: ($event.target as HTMLInputElement).value.trim() })"
        />
      </label>
      <label class="setting-row">
        <span>密码</span>
        <input
          :value="settings.smtp_password"
          type="password"
          placeholder="应用专用密码"
          @change="patch({ smtp_password: ($event.target as HTMLInputElement).value })"
        />
      </label>
      <label class="setting-row">
        <span>发件人</span>
        <input
          :value="settings.smtp_from"
          placeholder="you@example.com"
          @change="patch({ smtp_from: ($event.target as HTMLInputElement).value.trim() })"
        />
      </label>
      <label class="setting-row">
        <span>收件人</span>
        <input
          :value="settings.smtp_to"
          placeholder="you@example.com"
          @change="patch({ smtp_to: ($event.target as HTMLInputElement).value.trim() })"
        />
      </label>
      <div class="setting-row">
        <span>连通性</span>
        <button type="button" class="btn tiny" :disabled="mailTesting" @click="sendTestMail">
          {{ mailTesting ? '发送中…' : '发送测试邮件' }}
        </button>
      </div>
    </div>
  </div>
</template>
