<script setup lang="ts">
import { computed } from 'vue'
import ConfirmPopover from '@/components/base/ConfirmPopover.vue'
import Icon from '@/components/base/Icon.vue'
import IconBtn from '@/components/base/IconBtn.vue'
import TagChip from '@/components/tag/TagChip.vue'
import DueBadge from '@/components/todo/DueBadge.vue'
import PriorityBadge from '@/components/todo/PriorityBadge.vue'
import RemarkDisplay from '@/components/todo/RemarkDisplay.vue'
import RemindBadge from '@/components/todo/RemindBadge.vue'
import type { Priority, RemarkStyle, Todo } from '@/typings/domain'
import { highlight } from '@/utils/search'
import { isOverdue } from '@/utils/todo'

/**
 * 待办卡片（原型 todoItemHTML；父待办与子任务同构）。
 *
 * 结构：.todo-body > [✕, .todo-head > [折叠箭头, checkbox, .todo-main > [.todo-row, 备注文本行]], .todo-foot]
 * - 右上角 ✕：仅未完成项渲染，仅悬浮**本卡片自身内容区**时显示（CSS `.todo-item > .todo-body:hover`
 *   负责父子层级隔离，组件不持有 hover 状态）；
 * - 常显徽章区：逾期标记 / 所属日期（搜索态）/ 重复 / 备注图标；
 * - 底部一行（与 .todo-head 平级，撑满整行）：左侧标签 + 完成时间徽章（常显），
 *   右侧操作区 ⏰ / ＋子任务 / ✏️（悬浮显示）；已完成项只保留 ＋子任务；
 * - ＋子任务仅顶级且子任务少于 5 个时显示；已完成父级新建子任务后由后端自动恢复为未完成；
 * - 搜索态：正文命中片段以 <mark> 高亮。
 *
 * 已完成事项仅允许修改备注；恢复未完成必须二次确认（由调用方负责）。
 */
const props = withDefaults(
  defineProps<{
    todo: Todo
    /** 深度：0=顶级，1=子任务，用于层级渐变与树连接线。 */
    depth?: number
    /** 已有子任务数，决定「＋子任务」是否显示（上限 5，与后端 validate_parent 一致）。 */
    childCount?: number
    /** 是否有子任务，控制折叠箭头形态。 */
    hasChildren?: boolean
    collapsed?: boolean
    remarkStyle?: RemarkStyle
    /** 删除确认态。 */
    confirming?: boolean
    /** 搜索命中时展示所属日期徽章。 */
    dateChip?: string
    /** 搜索关键词：正文命中片段高亮。 */
    query?: string
    /** 搜索时命中的子任务：虚线框标出。 */
    searchHit?: boolean
  }>(),
  {
    depth: 0,
    childCount: 0,
    hasChildren: false,
    collapsed: false,
    remarkStyle: 'mixed',
    confirming: false,
    dateChip: '',
    query: '',
    searchHit: false,
  },
)

const emit = defineEmits<{
  (e: 'toggle-done'): void
  (e: 'toggle-collapse'): void
  (e: 'open-priority', anchor: HTMLElement): void
  (e: 'edit-due'): void
  (e: 'edit-remind'): void
  (e: 'edit-repeat', anchor: HTMLElement): void
  (e: 'edit'): void
  (e: 'add-sub'): void
  (e: 'ask-delete'): void
  (e: 'confirm-delete'): void
  (e: 'cancel-delete'): void
  (e: 'open-tags'): void
}>()

/** 子任务上限（原型 MAX_CHILDREN，与后端一致）。 */
const MAX_CHILDREN = 5
/** 混合模式下备注超过此字数改用图标徽章（与 RemarkDisplay 的阈值一致）。 */
const REMARK_MIXED_THRESHOLD = 100

const done = computed(() => props.todo.status === 'done')
const overdue = computed(() => isOverdue(props.todo))
const canEdit = computed(() => !done.value)
const canAddChild = computed(() => props.depth === 0 && props.childCount < MAX_CHILDREN)
/** 底栏是否渲染：有标签或有操作按钮（原型 tagsHtml || opsHtml）。 */
const showFoot = computed(() => props.todo.tags.length > 0 || canEdit.value || canAddChild.value)

/** 备注形态只选其一：图标进徽章区，文本行落在内容下方。 */
const remarkForm = computed<'icon' | 'text' | 'none'>(() => {
  if (!props.todo.remark) return 'none'
  if (props.remarkStyle === 'icon') return 'icon'
  if (props.remarkStyle === 'text') return 'text'
  return props.todo.remark.length > REMARK_MIXED_THRESHOLD ? 'icon' : 'text'
})

/** 重复规则的展示文案。 */
const repeatLabel = computed(() => {
  if (props.todo.repeat_rule === 'daily') return '每天'
  if (props.todo.repeat_rule === 'weekly') return '每周'
  return ''
})

/** 正文分段：命中片段渲染为 <mark>。 */
const textSegments = computed(() => highlight(props.todo.content, props.query))

const addChildTitle = computed(
  () => `添加子任务（${props.childCount}/${MAX_CHILDREN}）${done.value ? ' · 新建后自动恢复为未完成' : ''}`,
)

function onPriority(anchor: HTMLElement): void {
  emit('open-priority', anchor)
}
</script>

<template>
  <li
    class="todo-item"
    :class="{
      done,
      overdue,
      collapsed: props.collapsed,
      'has-children': props.hasChildren,
      [`depth-${props.depth}`]: props.depth > 0,
      'search-hit': props.searchHit,
    }"
  >
    <!-- todo-body 是悬浮判定范围：CSS 用直接子代选择器实现父子按钮隔离 -->
    <div class="todo-body">
      <ConfirmPopover
        v-if="props.confirming"
        :text="props.depth > 0 ? '⚠️ 确认删除该子任务？' : '⚠️ 确认删除该待办事项？'"
        @confirm="emit('confirm-delete')"
        @cancel="emit('cancel-delete')"
      />

      <!-- 右上角 ✕：已完成项禁删，不渲染 -->
      <button v-if="!done" type="button" class="card-close todo-del" title="删除待办" @click="emit('ask-delete')">
        <Icon name="close" />
      </button>

      <div class="todo-head">
        <!-- 折叠箭头：叶子节点渲染为占位以保持对齐 -->
        <span
          class="tree-toggle"
          :class="{ leaf: !props.hasChildren, closed: props.collapsed }"
          :title="props.collapsed ? '展开子任务' : '折叠子任务'"
          @click.stop="emit('toggle-collapse')"
          >▸</span
        >

        <span class="checkbox" :title="done ? '恢复为未完成（需确认）' : '标记完成'" @click.stop="emit('toggle-done')">
          <template v-if="done">✓</template>
        </span>

        <div class="todo-main">
          <div class="todo-row">
            <PriorityBadge :priority="props.todo.priority as Priority" :readonly="done" @open="onPriority" />
            <span class="todo-text">
              <template v-for="(segment, index) in textSegments" :key="index">
                <mark v-if="segment.hit">{{ segment.text }}</mark>
                <template v-else>{{ segment.text }}</template>
              </template>
            </span>

            <!-- 常显徽章区：不随 hover 隐藏，右侧留出 ✕ 的角落空间 -->
            <span class="todo-badges">
              <span v-if="props.dateChip" class="todo-date-chip" title="所属日期">{{ props.dateChip }}</span>
              <span v-if="overdue" class="overdue-flag" title="完成时间已过">逾期</span>
              <span
                v-if="repeatLabel"
                class="todo-meta repeat"
                :title="`重复提醒：${repeatLabel}（点击切换/结束）`"
                @click.stop="emit('edit-repeat', $event.currentTarget as HTMLElement)"
                ><span class="ix">🔁</span>{{ repeatLabel }}</span
              >
              <RemarkDisplay
                v-if="remarkForm === 'icon'"
                :remark="props.todo.remark"
                mode="icon"
                @edit="emit('edit')"
              />
            </span>
          </div>

          <!-- 置灰文本行形态的备注落在内容下方 -->
          <RemarkDisplay v-if="remarkForm === 'text'" :remark="props.todo.remark" mode="text" @edit="emit('edit')" />
        </div>
      </div>

      <!-- 底栏与 todo-head 平级：左侧常显（标签 + 完成时间），右侧悬浮显示的操作区 -->
      <div v-if="showFoot" class="todo-foot">
        <div class="todo-foot-left">
          <div v-if="props.todo.tags.length" class="todo-tags">
            <TagChip
              v-for="tag in props.todo.tags"
              :key="tag"
              class="todo-tag"
              :label="tag"
              :deletable="false"
              @click="emit('open-tags')"
            />
          </div>
          <DueBadge :due-at="props.todo.due_at" :overdue="overdue" :readonly="done" @edit="emit('edit-due')" />
        </div>

        <div v-if="canEdit || canAddChild" class="todo-ops">
          <RemindBadge
            v-if="canEdit"
            :offset-minutes="props.todo.remind_offset_minutes"
            :desktop="props.todo.remind_desktop"
            :email="props.todo.remind_email"
            @edit="emit('edit-remind')"
          />
          <IconBtn v-if="canAddChild" :title="addChildTitle" @click="emit('add-sub')">＋</IconBtn>
          <IconBtn v-if="canEdit" title="编辑内容" @click="emit('edit')"><Icon name="edit" /></IconBtn>
        </div>
      </div>
    </div>

    <!-- 子任务树由父组件通过插槽注入，保证连接线的 DOM 结构与原型一致 -->
    <slot name="children" />
  </li>
</template>
