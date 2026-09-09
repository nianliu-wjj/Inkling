<script setup lang="ts">
import { getTextFromHtml } from 'simple-mind-map/src/utils/index.js'
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { requireMindMap, useMindMap } from '../core/useMindMap'

/**
 * 搜索与替换浮层（移植参考端 Search.vue）。
 *
 * 订阅自定义事件 `showSearch`（NavigatorToolbar 发出）与 Ctrl+F 呼出；回车逐个定位命中，
 * 展示 `currentIndex / total` 与命中列表；支持替换当前 / 全部。命中列表、计数由库事件
 * `search_match_node_list_change` / `search_info_change` 驱动；点画布 / 节点 / 展开按钮时输入框失焦。
 */
const ctx = useMindMap()
const { bus, ui } = ctx

const show = ref(false)
const searchText = ref('')
const replaceText = ref('')
const showReplaceInput = ref(false)
const currentIndex = ref(0)
const total = ref(0)
const showSearchInfo = ref(false)
const showResultList = ref(false)
const resultList = ref<Array<{ id: string; text: string; name: string }>>([])
const searchInputRef = ref<HTMLInputElement | null>(null)

/** 空值判定（参考端库 isUndef，未在类型声明内，此处本地实现）。 */
function isUndef(value: string): boolean {
  return value === undefined || value === null || value === ''
}

watch(searchText, () => {
  if (isUndef(searchText.value)) {
    currentIndex.value = 0
    total.value = 0
    showSearchInfo.value = false
  }
})

function open(): void {
  ui.activeSidebar = ''
  show.value = true
  void nextTick(() => searchInputRef.value?.focus())
}

function close(): void {
  show.value = false
  showResultList.value = false
  showSearchInfo.value = false
  total.value = 0
  currentIndex.value = 0
  searchText.value = ''
  hideReplace()
  requireMindMap(ctx).search?.endSearch()
}

function hideReplace(): void {
  showReplaceInput.value = false
  replaceText.value = ''
}

/** 聚焦时禁止键入自动进入节点编辑，失焦恢复（参考端 onFocus/onBlur）。 */
function onFocus(): void {
  requireMindMap(ctx).updateConfig({ enableAutoEnterTextEditWhenKeydown: false })
}
function onBlur(): void {
  requireMindMap(ctx).updateConfig({ enableAutoEnterTextEditWhenKeydown: true })
}
function blurInputs(): void {
  searchInputRef.value?.blur()
}

function searchNext(): void {
  showResultList.value = true
  requireMindMap(ctx).search.search(searchText.value)
}
function replace(): void {
  requireMindMap(ctx).search.replace(replaceText.value, true)
}
function replaceAll(): void {
  requireMindMap(ctx).search.replaceAll(replaceText.value)
}
function jumpTo(index: number): void {
  requireMindMap(ctx).search.jump(index)
}

function onSearchInfoChange(data: { currentIndex: number; total: number }): void {
  currentIndex.value = data.currentIndex + 1
  total.value = data.total
  showSearchInfo.value = true
}

/** 命中列表变化：把匹配片段包成高亮 span（参考端 onSearchMatchNodeListChange）。 */
function onMatchListChange(list: Array<Record<string, unknown>>): void {
  const keyword = searchText.value.trim()
  resultList.value = list.map((item) => {
    const data = (item.data ?? (item.nodeData as { data: Record<string, unknown> })?.data ?? {}) as Record<
      string,
      unknown
    >
    let name = String(data.text ?? '')
    if (data.richText) name = getTextFromHtml(name)
    const text = keyword
      ? name.replace(
          new RegExp(keyword.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g'),
          (a) => `<span class="mm-search-hit">${a}</span>`,
        )
      : name
    return { id: String(data.uid ?? ''), text, name }
  })
}

const offs: Array<() => void> = []
onMounted(() => {
  offs.push(
    bus.on('showSearch', open),
    bus.on('search_info_change', (data) => onSearchInfoChange(data as { currentIndex: number; total: number })),
    bus.on('search_match_node_list_change', (list) => onMatchListChange(list as Array<Record<string, unknown>>)),
    bus.on('node_click', blurInputs),
    bus.on('draw_click', blurInputs),
    bus.on('expand_btn_click', blurInputs),
    bus.on('setData', close),
  )
  requireMindMap(ctx).keyCommand?.addShortcut?.('Control+f', open)
})
onBeforeUnmount(() => {
  offs.forEach((off) => off())
  ctx.mindMap.value?.keyCommand?.removeShortcut?.('Control+f', open)
})
</script>

<template>
  <div class="mm-search mm-panel" :class="{ show }">
    <button type="button" class="mm-search-close" title="关闭搜索" @click="close">✕</button>
    <div class="mm-search-input-box">
      <input
        ref="searchInputRef"
        v-model="searchText"
        class="mm-dialog-input mm-search-input"
        placeholder="搜索节点…"
        @keyup.enter.stop="searchNext"
        @keydown.stop
        @focus="onFocus"
        @blur="onBlur"
      />
      <button v-if="!isUndef(searchText)" type="button" class="btn tiny" @click="showReplaceInput = true">替换</button>
      <span v-if="showSearchInfo && !isUndef(searchText)" class="mm-search-info">{{ currentIndex }} / {{ total }}</span>
    </div>

    <div v-if="showReplaceInput" class="mm-search-replace-box">
      <input
        v-model="replaceText"
        class="mm-dialog-input mm-search-input"
        placeholder="替换为…"
        @keydown.stop
        @focus="onFocus"
        @blur="onBlur"
      />
      <button type="button" class="btn tiny" @click="hideReplace">取消</button>
    </div>
    <div v-if="showReplaceInput" class="mm-search-btns">
      <button type="button" class="btn tiny" :disabled="ui.isReadonly" @click="replace">替换</button>
      <button type="button" class="btn tiny" :disabled="ui.isReadonly" @click="replaceAll">全部替换</button>
    </div>

    <div v-if="showResultList" class="mm-search-result-list">
      <div
        v-for="(item, index) in resultList"
        :key="item.id"
        class="mm-search-result-item"
        :title="item.name"
        @click.stop="jumpTo(index)"
        v-html="item.text"
      />
      <div v-if="resultList.length === 0" class="mm-search-empty">无匹配结果</div>
    </div>
  </div>
</template>
