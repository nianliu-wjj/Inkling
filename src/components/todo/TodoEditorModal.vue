<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue'
import ModalShell from '@/components/base/ModalShell.vue'
import TagChip from '@/components/tag/TagChip.vue'
import { useShakeConfirm } from '@/composables/useShakeConfirm'
import { useToast } from '@/composables/useToast'
import { logger } from '@/service/logger'
import type { Priority, Todo, TodoInput } from '@/typings/domain'
import { DEFAULT_REMIND_OFFSET, REMIND_OPTIONS } from '@/constants/reminder'
import { fromDateAndTimeInputs, toDateAndTimeInputs, todayKey } from '@/utils/datetime'

/**
 * 待办编辑弹窗（创建 / 子任务 / 提醒 / 编辑 共用）。
 *
 * 需求 2.2 的约束：
 * - 字段：内容(必填) / 标签(≤3 个，每个 ≤10 字) / 备注(≤200 字带计数) /
 *   完成日期+时刻(必填) / 提醒日期+时间(选填) / 优先级；
 * - **提醒模式**下其余字段只读禁用，仅可修改提醒；
 * - 新建时完成时间不得早于当前时刻（日期下限锁今天），默认 = 当前 + 1 小时；
 *   编辑既有事项不设下限；历史日期补录允许并提示；
 * - 子任务完成时间不得晚于父待办（提交前拦截）。
 */
type Mode = 'create' | 'edit' | 'child'
type Focus = 'content' | 'due' | 'remind'

const props = withDefaults(
  defineProps<{
    mode: Mode
    /** 编辑既有事项时传入；创建时为空。 */
    todo?: Todo | null
    /** 新建子任务时的父待办，用于完成时间上限校验。 */
    parent?: Todo | null
    /** 归档页在历史日期新增时预填该日期（YYYY-MM-DD）。 */
    presetDate?: string
    focus?: Focus
  }>(),
  { todo: null, parent: null, presetDate: '', focus: 'content' },
)

const emit = defineEmits<{
  (e: 'save', input: TodoInput): void
  (e: 'close'): void
}>()

const { toast } = useToast()
const shake = useShakeConfirm()

/** 默认完成时间 = 当前 + 1 小时（需求指定）。 */
function defaultDue(): { date: string; time: string } {
  const target = new Date(Date.now() + 3_600_000)
  const value = toDateAndTimeInputs(target.toISOString())
  // 归档页在历史日期新增待办时，日期归入该日期（补录）。
  return props.presetDate ? { date: props.presetDate, time: value.time } : value
}

const initialDue = props.todo ? toDateAndTimeInputs(props.todo.due_at) : defaultDue()

const content = ref(props.todo?.content ?? '')
const contentInput = ref<HTMLInputElement | null>(null)
const dueTimeInput = ref<HTMLInputElement | null>(null)
const remindSelect = ref<HTMLSelectElement | null>(null)
const tags = ref<string[]>([...(props.todo?.tags ?? [])])
const tagInput = ref('')
const remark = ref(props.todo?.remark ?? '')
const dueDate = ref(initialDue.date)
const dueTime = ref(initialDue.time)
const remindOffset = ref<number | null>(props.todo ? props.todo.remind_offset_minutes : DEFAULT_REMIND_OFFSET)
const remindDesktop = ref(props.todo ? props.todo.remind_desktop : true)
const remindEmail = ref(props.todo ? props.todo.remind_email : false)
const priority = ref<Priority>((props.todo?.priority as Priority) ?? 'medium')

/** 已完成事项只允许编辑备注，约束由前后端共同执行。 */
const completedOnly = computed(() => props.todo?.status === 'done')
const fieldsDisabled = computed(() => completedOnly.value)
/** 新建（含子任务）时锁定日期下限为今天。 */
/** 选「不提醒」时渠道勾选无意义，禁用并置灰。 */
const channelsDisabled = computed(() => fieldsDisabled.value || remindOffset.value === null)
const isNew = computed(() => props.mode === 'create' || props.mode === 'child')
const minDate = computed(() => (isNew.value && !props.presetDate ? todayKey() : ''))

const title = computed(() => {
  switch (props.mode) {
    case 'create':
      return '✏️ 新增待办'
    case 'child':
      return '✏️ 新增子任务'
    default:
      return '✏️ 编辑待办'
  }
})

const hint = computed(() => {
  if (completedOnly.value) return '已完成待办仅允许修改备注'
  if (props.presetDate && props.presetDate < todayKey()) return '历史日期补录：将归入所选日期'
  if (props.mode === 'child') return '子任务的完成时间不能晚于父待办'
  return '完成时间为必填；提醒会在所选时间点与完成时间到点各触发一次'
})

onMounted(() => {
  void nextTick(() => {
    if (props.focus === 'due') dueTimeInput.value?.focus()
    else if (props.focus === 'remind') remindSelect.value?.focus()
    else contentInput.value?.focus()
  })
})

function addTag(): void {
  const name = tagInput.value.trim()
  if (!name) return
  if (tags.value.length >= 3) {
    toast('最多只能添加 3 个标签')
    return
  }
  if (name.length > 10) {
    toast('标签最多 10 个字')
    return
  }
  if (tags.value.includes(name)) {
    toast('该标签已存在')
    return
  }
  tags.value.push(name)
  tagInput.value = ''
}

function removeTag(tag: string): void {
  if (!shake.press(tag)) return
  tags.value = tags.value.filter((t) => t !== tag)
}

function save(): void {
  if (!content.value.trim()) {
    toast('待办内容不能为空')
    return
  }

  const dueAt = fromDateAndTimeInputs(dueDate.value, dueTime.value)
  if (!dueAt) {
    toast('请填写完整的完成日期与时刻')
    return
  }

  // 新建时完成时间不得早于当前（编辑既有事项不设此下限）。
  if (isNew.value && !props.presetDate && new Date(dueAt).getTime() < Date.now()) {
    toast('完成时间不能早于当前时刻')
    return
  }

  // 子任务不得晚于父待办（父级已完成时后端会豁免，此处只拦未完成父级）。
  if (props.parent && props.parent.status === 'open') {
    if (new Date(dueAt).getTime() > new Date(props.parent.due_at).getTime()) {
      toast('子任务的完成时间不能晚于父待办')
      return
    }
  }

  // 选了提醒时间却一个渠道都没勾，等于不会提醒，提前拦下避免误以为已生效。
  if (remindOffset.value !== null && !remindDesktop.value && !remindEmail.value) {
    toast('请至少选择一种提醒方式')
    return
  }

  const input: TodoInput = {
    id: props.todo?.id,
    content: content.value.trim(),
    dueAt,
    remindOffsetMinutes: remindOffset.value,
    remindDesktop: remindDesktop.value,
    remindEmail: remindEmail.value,
    repeatRule: props.todo?.repeat_rule ?? null,
    priority: priority.value,
    remark: remark.value,
    tags: [...tags.value],
    parentId: props.parent?.id ?? props.todo?.parent_id ?? null,
    // 补录历史日期时放行后端的「不得早于当前」校验。
    allowPast: Boolean(props.presetDate) || !isNew.value,
  }

  logger.info('todo-editor', `保存待办 mode=${props.mode}`, input)
  emit('save', input)
}
</script>

<template>
  <ModalShell overlay-id="todoEditorOverlay" modal-id="todoEditorModal" :title="title" @close="emit('close')">
    <input
      ref="contentInput"
      v-model="content"
      class="search-input"
      placeholder="待办内容…"
      :disabled="fieldsDisabled"
    />

    <div class="te-tags-row">
      <span class="te-label">标签</span>
      <div class="te-tags">
        <TagChip
          v-for="tag in tags"
          :key="tag"
          :label="tag"
          :shaking="shake.isArmed(tag)"
          :deletable="!fieldsDisabled"
          @remove="removeTag(tag)"
        />
        <span v-if="!tags.length" class="te-tags-empty">暂无标签</span>
      </div>
    </div>
    <input
      v-model="tagInput"
      class="search-input te-tag-input"
      placeholder="输入标签回车添加（最多 3 个 · 每个 10 字）"
      maxlength="10"
      :disabled="fieldsDisabled"
      @keydown.enter.prevent="addTag"
    />

    <div class="te-remark-wrap">
      <span class="te-label">备注</span>
      <textarea
        id="todoEditorRemark"
        v-model="remark"
        maxlength="200"
        rows="2"
        placeholder="补充说明…（选填，最多 200 字）"
      />
      <span class="te-remark-count">{{ remark.length }}/200</span>
    </div>

    <!-- 完成时间与优先级同属「必填的排期信息」，并排一行；提醒作为选填单独成行。 -->
    <div class="todo-editor-grid">
      <label class="te-field te-field-date">
        完成日期
        <input v-model="dueDate" type="date" :min="minDate" :disabled="fieldsDisabled" />
      </label>
      <label class="te-field te-field-time">
        完成时间
        <input ref="dueTimeInput" v-model="dueTime" type="time" :disabled="fieldsDisabled" />
      </label>
      <label class="te-field te-field-priority">
        优先级
        <select v-model="priority" :disabled="fieldsDisabled">
          <option value="high">🔴 高</option>
          <option value="medium">🟡 中</option>
          <option value="low">🟢 低</option>
        </select>
      </label>
    </div>

    <div class="todo-editor-grid">
      <label class="te-field te-field-date">
        提醒时间
        <select ref="remindSelect" v-model="remindOffset" :disabled="fieldsDisabled">
          <option v-for="option in REMIND_OPTIONS" :key="String(option.value)" :value="option.value">
            {{ option.label }}
          </option>
        </select>
      </label>
      <div class="te-field te-remind-channels">
        提醒方式
        <div class="te-channel-row">
          <label class="te-channel">
            <input v-model="remindDesktop" type="checkbox" :disabled="channelsDisabled" />
            桌面弹窗
          </label>
          <label class="te-channel">
            <input v-model="remindEmail" type="checkbox" :disabled="channelsDisabled" />
            邮箱
          </label>
        </div>
      </div>
    </div>

    <template #footer>
      <span class="clip-editor-hint">{{ hint }}</span>
      <div class="clip-editor-actions">
        <button type="button" class="btn ghost" @click="emit('close')">取消</button>
        <button type="button" class="btn primary" @click="save">保存</button>
      </div>
    </template>
  </ModalShell>
</template>
