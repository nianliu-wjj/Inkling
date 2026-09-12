/**
 * 待办编辑浮层的模式表（原型 app.js `TODO_SECTS` / `openTodoEditor` 的 titles）。
 *
 * 七种模式：create / edit / child 是全字段；due / remind / tags / remark 是聚焦单区段
 * （只显示对应 .te-sect，其余字段取待办原值原样提交）。抽成常量便于 TodoEditorPanel 与调用方共用类型。
 */

export type TodoEditorMode = 'create' | 'edit' | 'child' | 'due' | 'remind' | 'tags' | 'remark'

/** 面板内的六个区段，对应模板 `.te-sect[data-sect]`。 */
export type TodoEditorSect = 'text' | 'tags' | 'remark' | 'due' | 'remind' | 'prio'

const FULL: readonly TodoEditorSect[] = ['text', 'tags', 'remark', 'due', 'remind', 'prio']

/** 各模式显示的区段（原型 TODO_SECTS 逐字）。 */
export const TODO_SECTS: Record<TodoEditorMode, readonly TodoEditorSect[]> = {
  create: FULL,
  edit: FULL,
  child: FULL,
  due: ['due'],
  remind: ['remind'],
  tags: ['tags'],
  remark: ['remark'],
}

/** 标题（原型 titles）：emoji 走 .ix 单独渲染，文字部分在这里。 */
export const TODO_EDITOR_TITLES: Record<TodoEditorMode, { icon: string; text: string }> = {
  create: { icon: '＋', text: '新建待办' },
  child: { icon: '＋', text: '添加子任务' },
  remind: { icon: '⏰', text: '设置 / 更改提醒' },
  due: { icon: '📅', text: '修改完成时间' },
  edit: { icon: '✏️', text: '编辑待办' },
  tags: { icon: '🏷️', text: '编辑标签' },
  remark: { icon: '📄', text: '编辑备注' },
}
