import { computed, ref, type ComputedRef, type Ref } from 'vue'
import { useSettings } from '@/composables/useData'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import { api, type LauncherStatus } from '@/service/tauri'
import type { Settings } from '@/typings/domain'

/** 启动器文件扫描根目录（JSON 存于 Settings.launcher_roots）。 */
export interface LauncherRootRow {
  path: string
  depth: number
  excludes: string[]
}

/**
 * 启动器设置逻辑（主窗口启动台页使用）：索引状态与重建、全盘索引开关、额外排除目录、扫描根目录增删改。
 * 从偏好设置页抽出，`patch` 为调用方的统一保存入口（局部覆盖后整体写回）。
 */
export function useLauncherSettings(patch: (partial: Partial<Settings>) => Promise<void>): {
  launcherStatus: Ref<LauncherStatus | null>
  rebuilding: ComputedRef<boolean>
  refreshLauncherStatus: () => Promise<void>
  rebuildLauncher: () => Promise<void>
  toggleFullDiskIndex: (value: boolean) => Promise<void>
  setExtraExcludes: (value: string) => Promise<void>
  launcherRoots: ComputedRef<LauncherRootRow[]>
  newRootPath: Ref<string>
  addLauncherRoot: () => void
  removeLauncherRoot: (index: number) => void
  setRootDepth: (index: number, depth: number) => void
} {
  const { settings } = useSettings()
  const { toast } = useToast()

  /** 启动器索引状态（展示 + 重建）。 */
  const launcherStatus = ref<LauncherStatus | null>(null)
  const rebuilding = computed(() => launcherStatus.value?.rebuilding ?? false)

  async function refreshLauncherStatus(): Promise<void> {
    try {
      launcherStatus.value = await api.launcher.status()
    } catch (error) {
      logger.error('launcher-settings', '读取启动器状态失败', error)
    }
  }

  async function rebuildLauncher(): Promise<void> {
    try {
      await api.launcher.rebuild()
      toast('已开始重建索引')
      // 重建在后台，稍后刷新状态
      setTimeout(() => void refreshLauncherStatus(), 1500)
    } catch (error) {
      logger.error('launcher-settings', '重建索引失败', error)
      toast('重建索引失败')
    }
  }

  /** 切换全盘文件索引：持久化后立即重建（索引来源变化）。 */
  async function toggleFullDiskIndex(value: boolean): Promise<void> {
    await patch({ launcher_full_disk_index: value })
    void rebuildLauncher()
  }

  /** 修改额外排除目录：持久化（下次重建生效，用户可手动「立即重建」）。 */
  async function setExtraExcludes(value: string): Promise<void> {
    await patch({ launcher_extra_excludes: value })
  }

  /** 根目录：以 JSON 存 Settings.launcher_roots，UI 上按行编辑「路径 | 深度」。 */
  const launcherRoots = computed<LauncherRootRow[]>(() => {
    const raw = settings.value.launcher_roots.trim()
    if (!raw) return []
    try {
      const parsed = JSON.parse(raw) as LauncherRootRow[]
      return Array.isArray(parsed) ? parsed : []
    } catch {
      return []
    }
  })

  function saveLauncherRoots(rows: LauncherRootRow[]): void {
    void patch({ launcher_roots: JSON.stringify(rows) }).then(() => void rebuildLauncher())
  }

  const newRootPath = ref('')
  function addLauncherRoot(): void {
    const path = newRootPath.value.trim()
    if (!path) return
    saveLauncherRoots([...launcherRoots.value, { path, depth: 4, excludes: ['node_modules', '.git'] }])
    newRootPath.value = ''
  }
  function removeLauncherRoot(index: number): void {
    saveLauncherRoots(launcherRoots.value.filter((_, i) => i !== index))
  }
  function setRootDepth(index: number, depth: number): void {
    const clamped = Math.min(8, Math.max(1, Math.round(depth) || 4))
    saveLauncherRoots(launcherRoots.value.map((row, i) => (i === index ? { ...row, depth: clamped } : row)))
  }

  return {
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
  }
}
