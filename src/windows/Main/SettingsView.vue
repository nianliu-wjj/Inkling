<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useSettings } from '@/composables/useData'
import { useShortcutRecorder } from '@/composables/useShortcutRecorder'
import { useToast } from '@/composables/useToast'
import { useGlass } from '@/composables/useGlass'
import { useTheme } from '@/composables/useTheme'
import { glassLevels } from '@/constants/glass'
import { builtinPlugins, resolvePlugins, serializePlugins } from '@/panel-plugins'
import { themes } from '@/constants/themes'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { CollapsePolicy, GlassLevel, PanelPosition, RemarkStyle, Settings } from '@/typings/domain'

/**
 * 归档 · 偏好设置页。
 *
 * 行序与原型 #archive-settings 一致：失焦收起 → 粘贴板保留天数 → 开机自启 → 全局快捷键 →
 * 备注展示样式 → 主题；其后为项目扩展项：面板唤出位置 / 窗口毛玻璃 / 玻璃质感 / 数据目录 /
 * 面板插件 / 邮件提醒。灵动岛与启动器的设置分别位于灵动岛页与启动台页（原型位置）。
 *
 * 所有修改即时保存，并由后端广播 settings-changed 同步到其他窗口。
 */
const { settings, save } = useSettings()
const { applyTheme } = useTheme()
const { applyGlass } = useGlass()
const { toast } = useToast()

const themeMenuOpen = ref(false)
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
/** 面板全局快捷键录制：改绑成功后持久化到 settings.shortcut。 */
const { recording, start: startRecording } = useShortcutRecorder({
  label: '面板',
  apply: (combo) => api.shortcut.rebind(combo),
  persist: (applied) => patch({ shortcut: applied }),
})

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
        <span>全局快捷键（面板）</span>
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

      <div class="setting-row">
        <span>主题</span>
        <!-- 结构与原型 renderThemeDD 一致：色点 → 名称(.dd-name 撑满) → 箭头/勾选靠右（.dd-check 靠 margin-left:auto 排最后） -->
        <div class="theme-dd" :class="{ open: themeMenuOpen }">
          <button type="button" class="theme-dd-trigger" @click="themeMenuOpen = !themeMenuOpen">
            <span class="theme-dots">
              <i v-for="dot in currentTheme.dots" :key="dot" :style="{ background: dot }" />
            </span>
            <span class="dd-name">{{ currentTheme.label }}</span>
            <span class="dd-chevron">▾</span>
          </button>
          <div v-if="themeMenuOpen" class="theme-dd-menu">
            <div
              v-for="option in themes"
              :key="option.key"
              class="theme-dd-opt"
              :class="{ active: option.key === settings.theme }"
              @click="pickTheme(option.key)"
            >
              <span class="theme-dots">
                <i v-for="dot in option.dots" :key="dot" :style="{ background: dot }" />
              </span>
              <span class="dd-name">{{ option.label }}</span>
              <span class="dd-check">✓</span>
            </div>
          </div>
        </div>
      </div>

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
