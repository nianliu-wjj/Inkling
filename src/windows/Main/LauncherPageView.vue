<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useSettings } from '@/composables/useData'
import { useLauncherResults } from '@/composables/useLauncherResults'
import { useLauncherSettings } from '@/composables/useLauncherSettings'
import { useShortcutRecorder } from '@/composables/useShortcutRecorder'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { api } from '@/service/tauri'
import type { Settings } from '@/typings/domain'

/**
 * 归档 · 启动台页（原型 #archive-launcher）。
 *
 * 标题 + 说明 + 内嵌试用区（复用浮窗启动台的输入行 / 列表 / 页脚样式，结果走共享结果源
 * `useLauncherResults`，与浮窗启动台完全同源，新数据源在本页即可试用）
 * + 「启动台设置」：全局快捷键录制、五个检索范围开关、历史保留天数与已记录地址数、全盘索引开关、
 * 额外排除目录、索引状态与重建、扫描根目录、「呼出浮窗启动台」
 * （spec 4C §4.7；全盘索引等项目扩展行排在原型行之后）。
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

// 结果源与浮窗启动台同源（useLauncherResults + mergeLauncherResults），页面内可直接试用全部数据源。
const { query, results, active, runActive, onKeydown } = useLauncherResults({ scope: 'launcher-page' })

/** 点击条目：默认「打开」。 */
function run(index: number): void {
  void runActive(index)
}

/** 内嵌列表容器：方向键翻页后把选中项滚进视野用（原型 `$('launcherPageList')`）。 */
const list = ref<HTMLElement | null>(null)

/**
 * 选中项滚动进视野（原型 ↑↓ 分支里的 `scrollIntoView({ block: 'nearest' })`，`docs/app.js:3406`）。
 * 订阅 `active` 而不是写在键盘处理里：键盘在组合式 `useLauncherResults.onKeydown` 内，本页只能订阅结果。
 * 鼠标悬停同样会改 `active`，但悬停项本就在视野内，`nearest` 不会产生位移。
 * `block: 'nearest'` 只滚最近的这一个可滚动祖先（`.launcher-list`），不会带动整页。
 */
watch(active, () => {
  void nextTick(() => {
    // 空列表时不滚：此时 children[0] 是空态提示，且 active 必为 0。
    if (!results.value.length) return
    list.value?.children[active.value]?.scrollIntoView({ block: 'nearest' })
  })
})

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

/**
 * 页脚提示（原型 renderLauncherPageResults 的两态文案，`docs/app.js:3379` / `:3390`）。
 * 与浮窗**不同**，不要统一：页面是「点击直接执行」，浮窗那句才是「Esc 关闭」。
 */
const footer = computed(() =>
  results.value.length
    ? `↑↓ 导航 · ↵ 执行 · 点击直接执行 · 共 ${results.value.length} 项`
    : '↵ 回车搜索 · 点击条目直接执行',
)

/**
 * 空态文案（原型 `docs/app.js:3378`）：有查询词说明没搜到，回车会用默认浏览器搜该词
 * （`useLauncherResults` 的 Enter 分支对浮窗与页面一视同仁，本页确实会开浏览器）。
 * 「空查询且无结果」这一态原型到不了（空查询时应用 / 命令段非空），保留自造的「索引尚未就绪」
 * ——比显示「…搜索「」」诚实。
 */
const emptyHint = computed(() =>
  query.value.trim() ? `未找到匹配项，按 Enter 用默认浏览器搜索「${query.value.trim()}」` : '索引尚未就绪',
)

/** 呼出浮窗启动台：与 Alt+Space 同一入口。 */
async function showLauncher(): Promise<void> {
  try {
    await api.launcher.show()
  } catch (error) {
    logger.error('launcher-page', '呼出浮窗启动台失败', error)
    toast('呼出启动台失败')
  }
}

/** 已记录历史地址条数（原型 lpHistoryCount：`N 条（保留近 M 天 · 无痕访问不记录）`）。 */
const historyCount = ref(0)

async function refreshHistoryCount(): Promise<void> {
  try {
    historyCount.value = await api.browserHistory.count()
  } catch (error) {
    logger.error('launcher-page', '读取历史条数失败', error)
  }
}

/**
 * 历史保留天数：钳制到 1–365 后写库（后端会顺手清理过期记录），再刷新条数。
 * 钳制规则与原型一致（`docs/app.js` 的 lpHistoryRetention change 分支）：非法输入回落到默认 100。
 */
async function setHistoryRetention(raw: string): Promise<void> {
  const days = Math.min(365, Math.max(1, Number(raw) || 100))
  logger.info('launcher-page', `历史保留天数改为 ${days} 天（输入 ${raw}）`)
  await patch({ launcher_history_retention_days: days })
  await refreshHistoryCount()
  toast(`浏览器历史保留天数已设为 ${days} 天`)
}

const historyCountText = computed(
  () => `${historyCount.value} 条（保留近 ${settings.value.launcher_history_retention_days} 天 · 无痕访问不记录）`,
)

onMounted(() => {
  void refreshLauncherStatus()
  // 原型进入启动台页即 pruneBrowserHistory 并刷新计数；这里等价地在挂载时读一次
  // （保留天数在设置页改动后由 settings_save 触发的 prune 保证库是最新的）。
  void refreshHistoryCount()
})
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
          placeholder="在此试用：搜索应用 / 命令 / 笔记 / 待办 / 历史，= 开头打开计算器"
          spellcheck="false"
          autocomplete="off"
          @keydown="onKeydown"
        />
      </div>
      <div ref="list" class="launcher-list">
        <div
          v-for="(result, index) in results"
          :key="result.id"
          class="launcher-item"
          :class="{ active: index === active }"
          @mouseenter="active = index"
          @click="run(index)"
        >
          <span class="li-ico">{{ result.ico }}</span>
          <span class="li-name"
            >{{ result.name }}<small v-if="result.sub">{{ result.sub }}</small></span
          >
          <span class="li-cat">{{ result.cat }}</span>
        </div>
        <div v-if="!results.length" class="launcher-empty">{{ emptyHint }}</div>
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
      <!-- 检索范围（原型 #lpScopeApps..：浮窗与页面共用同一份偏好，改这里浮窗同步生效） -->
      <label class="setting-row">
        <span>检索范围：应用与命令</span>
        <input
          type="checkbox"
          :checked="settings.launcher_scope_apps"
          @change="patch({ launcher_scope_apps: ($event.target as HTMLInputElement).checked })"
        />
      </label>
      <label class="setting-row">
        <span>检索范围：笔记</span>
        <input
          type="checkbox"
          :checked="settings.launcher_scope_notes"
          @change="patch({ launcher_scope_notes: ($event.target as HTMLInputElement).checked })"
        />
      </label>
      <label class="setting-row">
        <span>检索范围：待办</span>
        <input
          type="checkbox"
          :checked="settings.launcher_scope_todos"
          @change="patch({ launcher_scope_todos: ($event.target as HTMLInputElement).checked })"
        />
      </label>
      <label class="setting-row">
        <span>计算器（= 开头，打开系统计算器）</span>
        <input
          type="checkbox"
          :checked="settings.launcher_scope_calc"
          @change="patch({ launcher_scope_calc: ($event.target as HTMLInputElement).checked })"
        />
      </label>
      <label class="setting-row">
        <span>检索范围：浏览器历史</span>
        <input
          type="checkbox"
          :checked="settings.launcher_scope_history"
          @change="patch({ launcher_scope_history: ($event.target as HTMLInputElement).checked })"
        />
      </label>
      <!-- 保留天数（原型 #lpHistoryRetention）：改小后后端在写库时顺手清理超期记录 -->
      <label class="setting-row">
        <span>历史保留天数</span>
        <input
          type="number"
          min="1"
          max="365"
          :value="settings.launcher_history_retention_days"
          @change="setHistoryRetention(($event.target as HTMLInputElement).value)"
        />
        天
      </label>
      <div class="setting-row">
        <span>已记录历史地址</span>
        <span class="clip-editor-hint">{{ historyCountText }}</span>
      </div>
      <!-- 本项目原有行；按 spec §4.7 / §3 #15「项目扩展行排在原型行之后」放在原型行（含本行）之后 -->
      <div class="setting-row">
        <span>浮窗启动台</span>
        <button type="button" class="btn tiny" @click="showLauncher">呼出浮窗启动台</button>
      </div>
      <!-- 以下为项目扩展行（原型没有），同样按「排在原型行之后」落在最后 -->
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
    </div>
  </div>
</template>
