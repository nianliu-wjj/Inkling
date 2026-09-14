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
 *
 * `patch` 的返回类型收成 `Promise<boolean>`（而不是 `unknown`）：本模块已有多处**依赖解析值的真值**
 * 决定要不要走后续流程（重建索引、返回是否落库）。放宽成 `unknown` 时编译器挡不住「传进来一个返回
 * `Promise<void>` 的 patch」——那种情况下这些分支会静默失效（`!(await patch(...))` 恒真 ⇒ 永不重建、
 * 永远报失败），是排查起来很贵的坑。收紧后由类型自证。
 */
export function useLauncherSettings(patch: (partial: Partial<Settings>) => Promise<boolean>): {
  launcherStatus: Ref<LauncherStatus | null>
  rebuilding: ComputedRef<boolean>
  refreshLauncherStatus: () => Promise<void>
  rebuildLauncher: () => Promise<void>
  toggleFullDiskIndex: (value: boolean) => Promise<boolean>
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

  /**
   * 切换全盘文件索引：持久化后立即重建（索引来源变化），返回是否已落库。
   *
   * 保存失败即中止并返回 false：`patch` 自己已弹「保存设置失败」，再往下走会拿**没生效**的旧配置
   * 去重建，还附送一条「已开始重建索引」——与 4C 验收记录 §5.1 那两处是同一类"失败仍报成功"。
   * 返回值交给调用方回退勾选框（`:checked` 绑定只在设置值**变化**时才更新 DOM，不回退就会
   * 「开关显示已开、实际没保存」）。
   */
  async function toggleFullDiskIndex(value: boolean): Promise<boolean> {
    if (!(await patch({ launcher_full_disk_index: value }))) return false
    void rebuildLauncher()
    return true
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

  /**
   * 保存根目录列表，返回是否已落库。
   *
   * 保存失败即中止并返回 false：`patch` 失败时自己已弹「保存设置失败」，原先仍继续
   * `rebuildLauncher()`，等于拿**没生效**的根目录配置重建一遍索引（4C 验收记录 §5.1）。
   * 返回值交给调用方判断该不该走后续的成功流程（当前是「新增根目录」用它决定是否清空输入框）。
   */
  async function saveLauncherRoots(rows: LauncherRootRow[]): Promise<boolean> {
    if (!(await patch({ launcher_roots: JSON.stringify(rows) }))) return false
    void rebuildLauncher()
    return true
  }

  const newRootPath = ref('')
  function addLauncherRoot(): void {
    const path = newRootPath.value.trim()
    if (!path) return
    void saveLauncherRoots([...launcherRoots.value, { path, depth: 4, excludes: ['node_modules', '.git'] }]).then(
      (saved) => {
        // 只在落库成功后清空输入框：失败时留着用户刚敲的路径，免得「行没加上、输入框却空了」。
        if (saved) newRootPath.value = ''
      },
    )
  }
  function removeLauncherRoot(index: number): void {
    void saveLauncherRoots(launcherRoots.value.filter((_, i) => i !== index))
  }
  function setRootDepth(index: number, depth: number): void {
    const clamped = Math.min(8, Math.max(1, Math.round(depth) || 4))
    void saveLauncherRoots(launcherRoots.value.map((row, i) => (i === index ? { ...row, depth: clamped } : row)))
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
