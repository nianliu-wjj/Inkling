<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, type CSSProperties } from 'vue'
import TagChip from '@/components/tag/TagChip.vue'
import { useShakeConfirm } from '@/composables/useShakeConfirm'
import { useToast } from '@/composables/useToast'
import { DEFAULT_REMIND_OFFSET, REMIND_OPTIONS, remindOffsetLabel } from '@/constants/reminder'
import { TODO_EDITOR_TITLES, TODO_SECTS, type TodoEditorMode, type TodoEditorSect } from '@/constants/todoEditor'
import { enter, exit } from '@/motion'
import { logger } from '@/service/logger'
import type { Priority, Todo, TodoInput } from '@/typings/domain'
import { anchorBeside, type Rect } from '@/utils/anchor'
import { formatDueLabel, fromDateAndTimeInputs, toDateAndTimeInputs, todayKey } from '@/utils/datetime'

/**
 * 待办编辑浮层（原型 #todoEditorOverlay > #todoEditorPanel，spec §4.3）。只在 editor 窗内渲染（EditorApp）。
 *
 * - 七种模式：create / edit / child 全字段；due / remind / tags / remark 只显示对应区段（TODO_SECTS），
 *   其余字段取待办原值原样提交；标题与底栏提示按模式切换；
 * - 定位：挂载后测自身尺寸，anchorBeside(anchor, align 'top', fallback 'center') 锚到卡片右侧（右侧不够翻左 .flip），
 *   箭头 --caret-y 对齐卡片中心；无锚点居中；入场横向 10px 滑入（--dur-base），退场淡出（--dur-fast）后 emit close；
 * - 关闭：✕ / 取消 / 点 overlay 空白 / Esc（自身 keydown capture）；
 * - 校验（需求 2.2）：只校验当前模式可见的区段；内容与完成时间必填；create / child 且非历史补录时不早于当前；
 *   子任务不晚于未完成父待办；标签 ≤3 · ≤10 字；备注 ≤200 字；选了提醒偏移必须至少勾一种渠道；已完成事项只允许改备注；
 * - 保存在途守卫：emit save 后锁住直到调用方 resetSaving()（失败）或窗口关闭（成功），防止回车 / 双击重复创建。
 */
const props = withDefaults(
  defineProps<{
    mode: TodoEditorMode
    /** 编辑既有事项时传入；创建时为空。 */
    todo?: Todo | null
    /** 子任务的父待办（新建子任务 / 编辑子任务），用于完成时间上限校验与提示。 */
    parent?: Todo | null
    /** 归档页在历史日期新增时预填该日期（YYYY-MM-DD）。 */
    presetDate?: string
    /** 锚点卡片矩形（本窗 CSS 像素）；null 居中。 */
    anchor?: Rect | null
  }>(),
  { todo: null, parent: null, presetDate: '', anchor: null },
)

const emit = defineEmits<{
  (e: 'save', input: TodoInput): void
  (e: 'close'): void
}>()

const { toast } = useToast()
const shake = useShakeConfirm()

// ── 区段与文案 ──

const sects = computed<readonly TodoEditorSect[]>(() => TODO_SECTS[props.mode])
function has(sect: TodoEditorSect): boolean {
  return sects.value.includes(sect)
}

const title = computed(() => TODO_EDITOR_TITLES[props.mode])
const isNew = computed(() => props.mode === 'create' || props.mode === 'child')

// ── 表单状态：全部从 todo 原值初始化，聚焦模式下未显示的字段保持原值提交 ──

/** 默认完成时间 = 当前 + 1 小时（需求指定）；历史补录时日期归入所选日。 */
function defaultDue(): { date: string; time: string } {
  const value = toDateAndTimeInputs(new Date(Date.now() + 3_600_000).toISOString())
  return props.presetDate ? { date: props.presetDate, time: value.time } : value
}

const initialDue = props.todo ? toDateAndTimeInputs(props.todo.due_at) : defaultDue()

const content = ref(props.todo?.content ?? '')
const tags = ref<string[]>([...(props.todo?.tags ?? [])])
const tagInput = ref('')
const remark = ref(props.todo?.remark ?? '')
const dueDate = ref(initialDue.date)
const dueTime = ref(initialDue.time)
const remindOffset = ref<number | null>(props.todo ? props.todo.remind_offset_minutes : DEFAULT_REMIND_OFFSET)
const remindDesktop = ref(props.todo ? props.todo.remind_desktop : true)
const remindEmail = ref(props.todo ? props.todo.remind_email : false)
const priority = ref<Priority>((props.todo?.priority as Priority) ?? 'medium')

const contentInput = ref<HTMLInputElement | null>(null)
const tagInputEl = ref<HTMLInputElement | null>(null)
const remarkInput = ref<HTMLTextAreaElement | null>(null)
const dueTimeInput = ref<HTMLInputElement | null>(null)
const remindSelect = ref<HTMLSelectElement | null>(null)

/** 已完成事项只允许编辑备注，约束由前后端共同执行。 */
const completedOnly = computed(() => props.todo?.status === 'done')
const fieldsDisabled = computed(() => completedOnly.value)
/** 选「不提醒」时渠道勾选无意义，禁用并置灰。 */
const channelsDisabled = computed(() => fieldsDisabled.value || remindOffset.value === null)
/** 新建（含子任务）且非历史补录时锁定日期下限为今天。 */
const minDate = computed(() => (isNew.value && !props.presetDate ? todayKey() : ''))
/** 子任务日期上限锁到未完成父待办的完成日（原型 dateEl.max）。 */
const maxDate = computed(() =>
  props.parent && props.parent.status === 'open' ? toDateAndTimeInputs(props.parent.due_at).date : '',
)

/** 底栏提示（原型 todoEditorHint，remind 文案按 D14 偏移口径）。 */
const hint = computed(() => {
  if (completedOnly.value) return '已完成待办仅允许修改备注'
  switch (props.mode) {
    case 'child':
      return `子任务完成时间不能晚于父待办（${props.parent ? formatDueLabel(props.parent.due_at) : '—'}）`
    case 'remind':
      return remindOffset.value === null
        ? '当前不提醒；选择偏移后可勾选桌面弹窗 / 邮箱'
        : `提醒在完成时间${remindOffsetLabel(remindOffset.value)}触发，可选桌面弹窗 / 邮箱`
    case 'due':
      return '完成时间决定列表排序与逾期判定；子任务不能晚于父待办'
    case 'tags':
      return '标签 ≤3 个、每个 ≤10 字；✕ 需再次点击确认删除'
    case 'remark':
      return '备注 ≤200 字'
    case 'create':
      return props.presetDate
        ? `完成时间默认 1 小时后，将归入 ${props.presetDate}${props.presetDate < todayKey() ? '（历史日期补录）' : ''}`
        : '完成时间默认 1 小时后，可修改'
    default:
      return '完成时间、任务内容必填'
  }
})

// ── 定位与入退场 ──

const panelRef = ref<HTMLElement | null>(null)
const left = ref(0)
const top = ref(0)
const caretY = ref(28)
const flip = ref(false)

const panelStyle = computed<CSSProperties>(() => ({
  left: `${left.value}px`,
  top: `${top.value}px`,
  '--caret-y': `${caretY.value}px`,
}))

/** 原型 positionTodoEditor：右侧 → 左翻 → 居中兜底，垂直对齐卡片顶部并防出屏。 */
function position(): void {
  const panel = panelRef.value
  if (!panel) return
  const result = anchorBeside(props.anchor, {
    size: { width: panel.offsetWidth || 340, height: panel.offsetHeight || 300 },
    viewport: { width: window.innerWidth, height: window.innerHeight },
    align: 'top',
    fallback: 'center',
  })
  left.value = result.left
  top.value = result.top
  caretY.value = result.caretY
  flip.value = result.placement === 'left'
  logger.debug('todo-editor', `定位 placement=${result.placement} left=${result.left} top=${result.top}`)
}

/** 按模式把焦点放到对应控件（原型 focusEl 表）。 */
function focusByMode(): void {
  const target: HTMLElement | null =
    props.mode === 'remind'
      ? remindSelect.value
      : props.mode === 'due'
        ? dueTimeInput.value
        : props.mode === 'tags'
          ? tagInputEl.value
          : props.mode === 'remark'
            ? remarkInput.value
            : contentInput.value
  target?.focus()
}

let closing = false

/** 退场淡出（原型 .12s）后再通知调用方关窗；重复触发只走一次。 */
async function close(): Promise<void> {
  if (closing) return
  closing = true
  logger.debug('todo-editor', '关闭浮层')
  if (panelRef.value) await exit(panelRef.value, { axis: 'x', distance: 0, duration: 'fast' })
  emit('close')
}

/** Esc 关闭；capture 保证优先于内部控件（如 select）的键盘处理。 */
function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    event.stopPropagation()
    void close()
  }
}

onMounted(async () => {
  document.addEventListener('keydown', onKeydown, true)
  const panel = panelRef.value
  if (panel) panel.style.opacity = '0' // 定位前不闪一帧在 (0,0)
  await nextTick()
  position()
  if (panel) void enter(panel, { axis: 'x', distance: flip.value ? 10 : -10, duration: 'base' })
  focusByMode()
})

onBeforeUnmount(() => document.removeEventListener('keydown', onKeydown, true))

// ── 标签 ──

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

// ── 保存 ──

/**
 * 保存在途守卫：内容框回车与底栏「保存」都会触发 save()，而调用方（EditorApp）保存成功后才关窗，
 * 期间若再次回车 / 双击会再 emit 一次 save，create / child 模式下就会重复建两条待办。
 * emit 前置 true；成功路径面板随窗口卸载无需复位，失败路径由调用方通过 resetSaving() 解锁以便修正后重试。
 */
const saving = ref(false)

/** 调用方保存失败时调用：解除在途守卫，让用户能修正后重新保存。 */
function resetSaving(): void {
  saving.value = false
  logger.debug('todo-editor', '保存失败，解除在途守卫')
}

defineExpose({ resetSaving })

function save(): void {
  // 已在保存中或正在退场时忽略重复触发。
  if (saving.value || closing) return

  // 校验只针对当前模式可见的区段（TODO_SECTS）；未显示的字段取待办原值原样提交，不再重复校验。
  if (has('text') && !content.value.trim()) {
    toast('待办内容不能为空')
    return
  }

  // 完整性检查不分模式：due 区段不可见时 dueAt 来自待办原值，必然合法；这里同时完成 TodoInput 所需的非空收窄。
  const dueAt = fromDateAndTimeInputs(dueDate.value, dueTime.value)
  if (!dueAt) {
    toast('请填写完整的完成日期与时刻')
    return
  }

  if (has('due')) {
    // 新建时完成时间不得早于当前（编辑既有事项不设此下限；历史补录例外）。
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
  }

  // 选了提醒时间却一个渠道都没勾，等于不会提醒，提前拦下避免误以为已生效。
  if (has('remind') && remindOffset.value !== null && !remindDesktop.value && !remindEmail.value) {
    toast('请至少选择一种提醒方式')
    return
  }

  // 标签上限（addTag 已逐个拦截，此处兜底整体校验）。
  if (has('tags') && (tags.value.length > 3 || tags.value.some((tag) => tag.length > 10))) {
    toast('标签最多 3 个、每个最多 10 字')
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
  saving.value = true
  emit('save', input)
}
</script>

<template>
  <!-- 透明 overlay 铺满 editor 窗：点空白关闭 -->
  <div id="todoEditorOverlay" @click.self="close">
    <div
      id="todoEditorPanel"
      ref="panelRef"
      class="glass"
      :class="{ flip }"
      :data-mode="props.mode"
      :style="panelStyle"
      role="dialog"
      aria-modal="true"
      :aria-label="title.text"
    >
      <div v-if="props.anchor" class="te-caret" />

      <div class="clip-editor-header">
        <span class="clip-editor-title"
          ><span class="ix">{{ title.icon }}</span> {{ title.text }}</span
        >
        <button type="button" class="icon-btn" title="关闭（Esc）" @click="close">
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path d="M1 1l10 10M11 1L1 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
          </svg>
        </button>
      </div>

      <div class="te-body">
        <!-- 任务内容（全字段模式） -->
        <div v-if="has('text')" class="te-sect" data-sect="text">
          <input
            ref="contentInput"
            v-model="content"
            class="search-input"
            placeholder="待办内容…"
            :disabled="fieldsDisabled"
            @keydown.enter.prevent="save"
          />
        </div>

        <!-- 标签（全字段 或 聚焦标签） -->
        <div v-if="has('tags')" class="te-sect" data-sect="tags">
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
            ref="tagInputEl"
            v-model="tagInput"
            class="search-input te-tag-input"
            placeholder="输入标签回车添加（最多 3 个 · 每个 10 字）"
            maxlength="10"
            :disabled="fieldsDisabled"
            @keydown.enter.prevent="addTag"
          />
        </div>

        <!-- 备注（全字段 或 聚焦备注） -->
        <div v-if="has('remark')" class="te-sect" data-sect="remark">
          <div class="te-remark-wrap">
            <span class="te-label">备注</span>
            <textarea
              id="todoEditorRemark"
              ref="remarkInput"
              v-model="remark"
              maxlength="200"
              rows="2"
              placeholder="补充说明…（选填，最多 200 字）"
            />
            <span class="te-remark-count">{{ remark.length }}/200</span>
          </div>
        </div>

        <!-- 完成时间（全字段 或 聚焦完成时间） -->
        <div v-if="has('due')" class="te-sect" data-sect="due">
          <div class="todo-editor-grid">
            <label class="te-field">
              完成日期
              <input v-model="dueDate" type="date" :min="minDate" :max="maxDate" :disabled="fieldsDisabled" />
            </label>
            <label class="te-field">
              完成时间
              <input ref="dueTimeInput" v-model="dueTime" type="time" :disabled="fieldsDisabled" />
            </label>
          </div>
        </div>

        <!-- 提醒（全字段 或 聚焦提醒）：D14 偏移下拉 + 渠道勾选 -->
        <div v-if="has('remind')" class="te-sect" data-sect="remind">
          <div class="todo-editor-grid">
            <label class="te-field">
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
        </div>

        <!-- 优先级（仅全字段） -->
        <div v-if="has('prio')" class="te-sect" data-sect="prio">
          <div class="todo-editor-grid">
            <label class="te-field">
              优先级
              <select v-model="priority" :disabled="fieldsDisabled">
                <option value="high">🔴 高</option>
                <option value="medium">🟡 中</option>
                <option value="low">🟢 低</option>
              </select>
            </label>
          </div>
        </div>
      </div>

      <div class="clip-editor-footer">
        <span class="clip-editor-hint">{{ hint }}</span>
        <div class="clip-editor-actions">
          <button type="button" class="btn ghost" @click="close">取消</button>
          <button type="button" class="btn primary" @click="save">保存</button>
        </div>
      </div>
    </div>
  </div>
</template>
