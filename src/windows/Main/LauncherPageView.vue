<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useSettings } from '@/composables/useData'
import { LAUNCHER_KIND_ICON, LAUNCHER_KIND_LABEL, useLauncherSearch } from '@/composables/useLauncherSearch'
import { useLauncherSettings } from '@/composables/useLauncherSettings'
import { useShortcutRecorder } from '@/composables/useShortcutRecorder'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { Settings } from '@/typings/domain'

/**
 * 归档 · 启动台页（原型 #archive-launcher）。
 *
 * 标题 + 说明 + 内嵌试用区（复用浮窗启动台的输入行 / 列表 / 页脚样式，数据走同一份 Rust 索引）
 * + 「启动台设置」：全局快捷键录制、全盘索引开关、额外排除目录、索引状态与重建、扫描根目录、
 * 「呼出浮窗启动台」。原型中依赖笔记 / 待办 / 计算器 / 浏览器历史检索的控件，当前后端没有这些
 * 检索源，不渲染（spec D10，留阶段四）。
 */
const { settings, save } = useSettings()
const { toast } = useToast()

/** 统一的保存入口：局部覆盖后整体写回。 */
async function patch(partial: Partial<Settings>): Promise<void> {
  const next: Settings = { ...settings.value, ...partial }
  try {
    await save(next)
  } catch {
    toast('保存设置失败')
  }
}

const { query, hits, active, run, onKeydown } = useLauncherSearch()
const {
  launcherStatus,
  rebuilding,
  refreshLauncherStatus,
  rebuildLauncher,
  toggleFullDiskIndex,
  setExtraExcludes,
  launcherRoots,
  newRootPath,
  addLauncherRoot,
  removeLauncherRoot,
  setRootDepth,
} = useLauncherSettings(patch)

const { recording, start: startRecording } = useShortcutRecorder({
  label: '启动器',
  apply: (combo) => api.launcher.rebindShortcut(combo),
  persist: (applied) => patch({ launcher_shortcut: applied }),
})

/** 页面说明：放在脚本里拼接，避免模板换行在中文之间引入空格。 */
const legend = computed(
  () =>
    `键盘优先的全局启动器：${settings.value.launcher_shortcut} 呼出浮窗；本地检索程序、UWP 应用、文件与文件夹，` +
    '支持拼音 / 首字母 / 拼写纠错，不联网、不上传',
)

/** 页脚提示（原型 renderLauncherPageResults）。 */
const footer = computed(() =>
  hits.value.length ? `↑↓ 导航 · ↵ 执行 · 点击直接执行 · 共 ${hits.value.length} 项` : '输入关键词开始搜索',
)

const emptyHint = computed(() => (query.value.trim() ? `未找到匹配「${query.value.trim()}」的项` : '索引尚未就绪'))

/** 呼出浮窗启动台：与 Alt+Space 同一入口。 */
async function showLauncher(): Promise<void> {
  try {
    await api.launcher.show()
  } catch (error) {
    logger.error('launcher-page', '呼出浮窗启动台失败', error)
    toast('呼出启动台失败')
  }
}

onMounted(() => void refreshLauncherStatus())
</script>

<template>
  <div class="archive-page">
    <div class="page-title">🚀 启动台</div>
    <div class="stats-legend">{{ legend }}</div>

    <!-- 内嵌试用区：复用浮窗启动台的输入行 / 列表 / 页脚样式 -->
    <div class="launcher-page-hero">
      <div class="launcher-input-row">
        <span class="launcher-ico">🚀</span>
        <input
          id="launcherPageInput"
          v-model="query"
          placeholder="在此试用：搜索程序 / 文件 / 文件夹，支持拼音与首字母"
          spellcheck="false"
          autocomplete="off"
          @keydown="onKeydown"
        />
      </div>
      <div class="launcher-list">
        <template v-if="hits.length">
          <div
            v-for="(hit, index) in hits"
            :key="hit.id"
            class="launcher-item"
            :class="{ active: index === active }"
            @mouseenter="active = index"
            @click="run(index)"
          >
            <span class="li-ico">{{ LAUNCHER_KIND_ICON[hit.kind] }}</span>
            <span class="li-name"
              >{{ hit.name }}<small v-if="hit.kind !== 'command'">{{ hit.path }}</small></span
            >
            <span class="li-cat">{{ LAUNCHER_KIND_LABEL[hit.kind] }}</span>
          </div>
        </template>
        <div v-else class="launcher-empty">{{ emptyHint }}</div>
      </div>
      <div class="launcher-footer">
        <span>{{ footer }}</span>
      </div>
    </div>

    <div class="page-title page-sub-title">⚙️ 启动台设置</div>
    <div class="settings-body">
      <!-- 含按钮的行不用 label 包裹：label 会把点击转发给内部控件 -->
      <div class="setting-row">
        <span>全局呼出快捷键</span>
        <kbd>{{ recording ? '按下组合键…' : settings.launcher_shortcut }}</kbd>
        <button type="button" class="btn tiny" :disabled="recording" @click="startRecording">重新录制</button>
      </div>
      <label class="setting-row">
        <span>全盘文件索引（搜索所有文件 / 文件夹）</span>
        <input
          type="checkbox"
          :checked="settings.launcher_full_disk_index"
          @change="toggleFullDiskIndex(($event.target as HTMLInputElement).checked)"
        />
      </label>
      <div class="setting-col">
        <span class="setting-col-label">额外排除目录（逗号分隔目录名）</span>
        <input
          class="search-input"
          type="text"
          :value="settings.launcher_extra_excludes"
          placeholder="如 tmp, backup, dist"
          @change="setExtraExcludes(($event.target as HTMLInputElement).value)"
        />
      </div>
      <div class="setting-row">
        <span>索引</span>
        <span class="clip-editor-hint">
          {{ launcherStatus ? `${launcherStatus.count} 项` : '加载中…' }}{{ rebuilding ? ' · 重建中' : '' }}
        </span>
        <button type="button" class="btn tiny" :disabled="rebuilding" @click="rebuildLauncher">立即重建</button>
      </div>
      <div class="setting-col">
        <span class="setting-col-label">文件扫描目录</span>
        <div class="launcher-roots">
          <div v-for="(root, index) in launcherRoots" :key="index" class="launcher-root-row">
            <span class="launcher-root-path" :title="root.path">{{ root.path }}</span>
            <input
              class="launcher-root-depth"
              type="number"
              min="1"
              max="8"
              :value="root.depth"
              title="扫描深度"
              @change="setRootDepth(index, Number(($event.target as HTMLInputElement).value))"
            />
            <button type="button" class="btn tiny" @click="removeLauncherRoot(index)">移除</button>
          </div>
          <div v-if="!launcherRoots.length" class="clip-editor-hint">未配置，使用默认（文档 / 下载 / 桌面 / 图片）</div>
          <div class="launcher-root-add">
            <input
              v-model="newRootPath"
              class="search-input"
              placeholder="粘贴文件夹绝对路径后点添加"
              @keydown.enter="addLauncherRoot"
            />
            <button type="button" class="btn tiny" @click="addLauncherRoot">添加</button>
          </div>
        </div>
      </div>
      <div class="setting-row">
        <span>浮窗启动台</span>
        <button type="button" class="btn tiny" @click="showLauncher">呼出浮窗启动台</button>
      </div>
    </div>
  </div>
</template>
