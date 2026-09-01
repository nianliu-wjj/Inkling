<script setup lang="ts">
import { computed } from 'vue'
import ConfirmPopover from '@/components/base/ConfirmPopover.vue'
import IconBtn from '@/components/base/IconBtn.vue'
import TagChip from '@/components/tag/TagChip.vue'
import DueBadge from '@/components/todo/DueBadge.vue'
import PriorityBadge from '@/components/todo/PriorityBadge.vue'
import RemarkDisplay from '@/components/todo/RemarkDisplay.vue'
import RemindBadge from '@/components/todo/RemindBadge.vue'
import type { Priority, RemarkStyle, Todo } from '@/typings/domain'
import { isOverdue } from '@/utils/todo'

/**
 * 待办卡片（父待办与子任务同构）。
 *
 * 需求 2.2 的布局约定：
 * - 右上角 ✕：仅悬浮**本卡片自身内容区**时显示（CSS `.todo-item > .todo-body:hover`
 *   负责父子层级隔离，组件不持有 hover 状态）；
 * - 常显徽章区：逾期标记 / 重复 / 所属日期 / 备注图标；
 * - 底部一行：左侧标签 + 完成时间徽章（常显），右侧操作区 ⏰/＋子任务/✏️（悬浮显示）；
 * - 子任务与父级同构，仅不显示「＋子任务」。
 *
 * **已完成事项仅允许修改备注；恢复未完成必须二次确认。
 */
const props = withDefaults(
  defineProps<{
    todo: Todo
    /** 深度：0=顶级，1=子任务，用于层级渐变与树连接线。 */
    depth?: number
    /** 是否为父级（决定是否显示「＋子任务」）。 */
    isParent?: boolean
    /** 是否有子任务，控制折叠箭头形态。 */
    hasChildren?: boolean
    collapsed?: boolean
    remarkStyle?: RemarkStyle
    /** 删除确认态。 */
    confirming?: boolean
    /** 搜索命中时展示所属日期徽章。 */
    dateChip?: string
    searchHit?: boolean
  }>(),
  {
    depth: 0,
    isParent: false,
    hasChildren: false,
    collapsed: false,
    remarkStyle: 'mixed',
    confirming: false,
    dateChip: '',
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

const done = computed(() => props.todo.status === 'done')
const overdue = computed(() => isOverdue(props.todo))

/** 重复规则的展示文案。 */
const repeatLabel = computed(() => {
  if (props.todo.repeat_rule === 'daily') return '🔁 每天'
  if (props.todo.repeat_rule === 'weekly') return '🔁 每周'
  return ''
})

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

      <!-- 右上角 ✕：占据一个角落位置，悬浮本卡片时显示 -->
      <IconBtn variant="todo-del" title="删除" @click="emit('ask-delete')">✕</IconBtn>

      <div class="todo-head">
        <!-- 折叠箭头：叶子节点渲染为占位以保持对齐 -->
        <span
          class="tree-toggle"
          :class="{ leaf: !props.hasChildren }"
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
            <span class="todo-text"
              ><slot name="text">{{ props.todo.content }}</slot></span
            >

            <!-- 常显徽章区：不随 hover 隐藏，右侧留出 ✕ 的角落空间 -->
            <span class="todo-badges">
              <span v-if="overdue" class="overdue-flag">逾期</span>
              <span v-if="props.dateChip" class="todo-date-chip">{{ props.dateChip }}</span>
              <span
                v-if="repeatLabel"
                class="repeat"
                title="点击修改重复方式"
                @click.stop="emit('edit-repeat', $event.currentTarget as HTMLElement)"
                >{{ repeatLabel }}</span
              >
              <RemarkDisplay
                v-if="props.remarkStyle !== 'text'"
                :remark="props.todo.remark"
                :mode="props.remarkStyle === 'icon' ? 'icon' : 'mixed'"
                @edit="emit('edit')"
              />
            </span>
          </div>

          <!-- 置灰文本行形态的备注落在内容下方 -->
          <RemarkDisplay
            v-if="props.remarkStyle !== 'icon'"
            :remark="props.todo.remark"
            :mode="props.remarkStyle === 'text' ? 'text' : 'mixed'"
            @edit="emit('edit')"
          />

          <div class="todo-foot">
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

            <!-- 悬浮才显示的操作区 -->
            <div class="todo-ops">
              <RemindBadge
                :remind-at="props.todo.remind_at"
                :due-at="props.todo.due_at"
                :readonly="done"
                @edit="emit('edit-remind')"
              />
              <IconBtn v-if="props.isParent" title="新增子任务" @click="emit('add-sub')">＋</IconBtn>
              <IconBtn title="编辑" @click="emit('edit')">✏️</IconBtn>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 子任务树由父组件通过插槽注入，保证连接线的 DOM 结构与原型一致 -->
    <slot name="children" />
  </li>
</template>
