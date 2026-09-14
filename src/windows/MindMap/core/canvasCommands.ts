import { logger } from '@/service/logger'
import { requireMindMap, type MindMapContext } from './useMindMap'

/**
 * 画布级命令的共享入口（阶段五 Task 5）。
 *
 * 「适应画布 / 展开全部 / 收起全部」在原型底栏与右键菜单里各有一个入口（`docs/index.html:429-431`
 * 与画布菜单），两处必须是**同一条命令**、同一种参数口径。原先只有 `popups/ContextMenu.vue` 内联
 * 写了这三条（`exec()` 的 switch 分支），底栏补齐时若再抄一份参数形状就会两处漂移——库这两个命令的
 * 参数还都是位置参数、含义不自明（见下）。故抽到这里，两个入口共用。
 *
 * 库侧签名（`simple-mind-map/src/core/render/Render.js`）：
 * - `EXPAND_ALL`       → `expandAllNode(uid = '')`：uid 非空时表示「从这个节点往下都允许展开」；
 * - `UNEXPAND_ALL`     → `unexpandAllNode(isSetRootNodeCenter = true, uid = '')`：第一个参数是
 *   **是否顺便把根节点居中**，第二个才是 uid 范围；
 * - 自适应不走 `execCommand`，是 `view.fit()`（与右键菜单「适应画布」一致）。
 *
 * 两处入口的差别只在 uid：右键菜单点在某个节点上时把该节点 uid 传下去（只影响它下面），
 * 从底栏按没有「当前节点」这个概念，一律传空串 = 全图。
 */

/** 适应画布：缩放到能看全所有节点（原型 `#mindmapFit`；右键菜单同名项）。 */
export function fitCanvas(ctx: MindMapContext): void {
  requireMindMap(ctx).view.fit()
  logger.info('mindmap', '画布命令：适应画布')
}

/**
 * 展开全部。`uid` 为空表示全图（底栏口径）；非空表示从该节点往下（右键菜单点节点时的口径）。
 */
export function expandAll(ctx: MindMapContext, uid = ''): void {
  ctx.bus.emit('execCommand', 'EXPAND_ALL', uid)
  logger.info('mindmap', `画布命令：展开${uid ? '该节点以下' : '全部'}节点`)
}

/**
 * 收起全部。库的第一个参数是「是否把根节点居中」——底栏与画布菜单都传 `true`
 * （`uid` 为空时 `!uid` 即 `true`），右键菜单点在某节点上时传 `false`，免得每收一次就把视口拽回根节点。
 */
export function unexpandAll(ctx: MindMapContext, uid = ''): void {
  ctx.bus.emit('execCommand', 'UNEXPAND_ALL', !uid, uid)
  logger.info('mindmap', `画布命令：收起${uid ? '该节点以下' : '全部'}节点`)
}
