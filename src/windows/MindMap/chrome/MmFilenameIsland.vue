<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'
import { logger } from '@/service/logger'
import { MAP_NAME_LIMIT, normalizeMapName } from '@/utils/mindmapName'

/**
 * 文件名岛（原型 `#mmFilenameIsland`，`docs/index.html:333-346`）。
 *
 * 形态照原型：常态是文本 + ✏️ 按钮，编辑态是输入框 + 💾 按钮；Enter / 失焦 / 💾 提交，
 * Esc 取消。**不渲染后缀徽章与后缀下拉**（spec D44）：我们的导图存进笔记
 * （`notes.content` 即导图名、`mindmap_data` 是导图数据），没有「保存格式」概念，
 * 原型那 4 个后缀实测也只是字符串、落库无差异。
 *
 * 🏷 是本项目的扩展（原型无对应物）：窗口内那条 `header.mindmap-bar` 删掉后，标签管理器
 * 只剩这一个入口——它原先是头部条上的 `TagList` 预览。常态与编辑态**都**放这枚按钮，
 * 否则一点 ✏️ 进入改名态，标签入口就凭空消失（改到一半想开标签管理器得先提交或 Esc 退出）。
 */
const props = defineProps<{ name: string; tags: string[] }>()
const emit = defineEmits<{ rename: [name: string]; 'open-tags': [] }>()

const editing = ref(false)
const draft = ref('')
const input = ref<HTMLInputElement | null>(null)

/** 🏷 的 title：有标签就列出来，没有时给动作名——单看一枚图标不知道点开是什么。 */
const tagsTitle = computed(() => (props.tags.length ? `标签：${props.tags.join('、')}` : '编辑标签'))

async function startEdit(): Promise<void> {
  draft.value = props.name
  editing.value = true
  await nextTick()
  input.value?.focus()
  input.value?.select()
  logger.debug('mindmap', '文件名岛进入编辑态')
}

function commit(): void {
  if (!editing.value) return
  const name = normalizeMapName(draft.value)
  editing.value = false
  if (name === props.name) return
  logger.info('mindmap', `导图改名为「${name}」`)
  emit('rename', name)
}

function cancel(): void {
  editing.value = false
  logger.debug('mindmap', '文件名岛放弃改名')
}

/**
 * 打开标签管理器。
 *
 * 编辑态下点它会先触发输入框的 `blur` → `commit()`（改名照常提交），再走到这里——
 * 这是刻意的：不给这里加 `@mousedown.prevent`，让行为与「点窗口其它任何地方」一致，
 * 也免得标签弹窗盖上去了、底下的输入框还停在编辑态。先提交改名、再开弹窗，两者互不干扰。
 */
function openTags(): void {
  logger.debug('mindmap', '文件名岛打开标签管理器')
  emit('open-tags')
}

/** Enter 提交；组合期间（中文输入法选词）不提交，见 `useLauncherResults` 的同款守卫。 */
function onEnter(event: KeyboardEvent): void {
  // 中文输入法：Enter 用于确认候选词（组合期间 isComposing 为真，部分环境只有 keyCode 229），
  // 此时不应提交——否则 v-model 在组合期间不更新 draft，会把名字静默写成「未命名导图」。
  if (event.isComposing || event.keyCode === 229) return
  commit()
}

/** Esc 取消改名；组合期间（中文输入法）Esc 是「取消候选词」，不当作放弃改名。 */
function onEsc(event: KeyboardEvent): void {
  // 与 onEnter 同源的守卫：组合期间 Esc 先被输入法消费，此时取消改名会连带丢掉候选词。
  if (event.isComposing || event.keyCode === 229) return
  cancel()
}
</script>

<template>
  <div class="mm-filename-island" @keydown.esc.stop="onEsc">
    <template v-if="editing">
      <input
        ref="input"
        v-model="draft"
        class="mm-filename-input"
        :maxlength="MAP_NAME_LIMIT"
        spellcheck="false"
        autocomplete="off"
        @keydown.enter.prevent="onEnter"
        @blur="commit"
      />
      <button
        type="button"
        class="mm-filename-btn"
        title="保存名称"
        aria-label="保存名称"
        @mousedown.prevent
        @click="commit"
      >
        💾
      </button>
      <button type="button" class="mm-filename-btn" :title="tagsTitle" aria-label="管理标签" @click="openTags">
        🏷
      </button>
    </template>
    <template v-else>
      <span class="mm-filename-text" title="点击编辑导图名" @click="startEdit">{{ name }}</span>
      <button type="button" class="mm-filename-btn" title="编辑导图名" aria-label="编辑导图名" @click="startEdit">
        ✏️
      </button>
      <button type="button" class="mm-filename-btn" :title="tagsTitle" aria-label="管理标签" @click="openTags">
        🏷
      </button>
    </template>
  </div>
</template>
