<script setup lang="ts">
import { NPopover } from 'naive-ui'
import { onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { requireMindMap, useMindMap } from '../core/useMindMap'
import { alignList, colorList, fontFamilyList, fontSizeList } from '../constants'

/**
 * 富文本浮动工具栏（移植参考端 RichTextToolbar.vue）。
 *
 * 订阅库事件 `rich_text_selection_change(hasRange, rect, formatInfo)`：有选区时按选区
 * 矩形定位并回显当前格式，无选区时隐藏。按钮走 `mindMap.richText.formatText({...})` 与
 * `removeFormat()`。工具栏 Teleport 到 body，避免被画布容器的 overflow/transform 裁剪。
 */
const ctx = useMindMap()
const { bus } = ctx

interface FormatInfo {
  bold?: boolean
  italic?: boolean
  underline?: boolean
  strike?: boolean
  font?: string
  size?: string
  color?: string
  background?: string
  align?: string
}

const show = ref(false)
const style = reactive<{ left: string; top: string }>({ left: '0', top: '0' })
const formatInfo = ref<FormatInfo>({})

function onSelectionChange(hasRange: boolean, rect: DOMRect | null, info: FormatInfo | null): void {
  if (hasRange && rect) {
    style.left = rect.left + rect.width / 2 + 'px'
    style.top = rect.top - 60 + 'px'
    formatInfo.value = { ...(info ?? {}) }
  }
  show.value = hasRange
}

/** 统一走 formatText；同时本地回显（参考端逐项 toggle）。 */
function format(patch: FormatInfo): void {
  formatInfo.value = { ...formatInfo.value, ...patch }
  requireMindMap(ctx).richText.formatText(patch)
}

const toggleBold = (): void => format({ bold: !formatInfo.value.bold })
const toggleItalic = (): void => format({ italic: !formatInfo.value.italic })
const toggleUnderline = (): void => format({ underline: !formatInfo.value.underline })
const toggleStrike = (): void => format({ strike: !formatInfo.value.strike })
const changeFont = (font: string): void => format({ font })
const changeSize = (size: number): void => format({ size: size + 'px' })
const changeColor = (color: string): void => format({ color })
const changeBackground = (background: string): void => format({ background })
const changeAlign = (align: string): void => format({ align })
function removeFormat(): void {
  requireMindMap(ctx).richText.removeFormat()
}

const offs: Array<() => void> = []
onMounted(() => {
  offs.push(
    bus.on('rich_text_selection_change', (hasRange, rect, info) =>
      onSelectionChange(hasRange as boolean, rect as DOMRect | null, info as FormatInfo | null),
    ),
  )
})
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <Teleport to="body">
    <div v-show="show" class="mm-rtt mm-panel" :style="style" @click.stop>
      <button type="button" class="mm-rtt-btn" :class="{ active: formatInfo.bold }" title="加粗" @click="toggleBold">
        <b>B</b>
      </button>
      <button
        type="button"
        class="mm-rtt-btn"
        :class="{ active: formatInfo.italic }"
        title="斜体"
        @click="toggleItalic"
      >
        <i>I</i>
      </button>
      <button
        type="button"
        class="mm-rtt-btn"
        :class="{ active: formatInfo.underline }"
        title="下划线"
        @click="toggleUnderline"
      >
        <u>U</u>
      </button>
      <button
        type="button"
        class="mm-rtt-btn"
        :class="{ active: formatInfo.strike }"
        title="删除线"
        @click="toggleStrike"
      >
        <s>S</s>
      </button>

      <!-- 字体 -->
      <NPopover trigger="hover" placement="bottom">
        <template #trigger>
          <button type="button" class="mm-rtt-btn" title="字体">字</button>
        </template>
        <div class="mm-rtt-options">
          <div
            v-for="item in fontFamilyList"
            :key="item.value"
            class="mm-rtt-option"
            :class="{ active: formatInfo.font === item.value }"
            :style="{ fontFamily: item.value }"
            @click="changeFont(item.value)"
          >
            {{ item.name }}
          </div>
        </div>
      </NPopover>

      <!-- 字号 -->
      <NPopover trigger="hover" placement="bottom">
        <template #trigger>
          <button type="button" class="mm-rtt-btn" title="字号">A±</button>
        </template>
        <div class="mm-rtt-options">
          <div
            v-for="size in fontSizeList"
            :key="size"
            class="mm-rtt-option"
            :class="{ active: formatInfo.size === size + 'px' }"
            @click="changeSize(size)"
          >
            {{ size }}px
          </div>
        </div>
      </NPopover>

      <!-- 字体颜色 -->
      <NPopover trigger="hover" placement="bottom">
        <template #trigger>
          <button type="button" class="mm-rtt-btn" title="文字颜色" :style="{ color: formatInfo.color }">A</button>
        </template>
        <div class="mm-rtt-colors">
          <button
            v-for="c in colorList"
            :key="c"
            type="button"
            class="mm-rtt-color"
            :class="{ transparent: c === 'transparent' }"
            :style="c === 'transparent' ? undefined : { background: c }"
            @click="changeColor(c)"
          />
        </div>
      </NPopover>

      <!-- 背景颜色 -->
      <NPopover trigger="hover" placement="bottom">
        <template #trigger>
          <button type="button" class="mm-rtt-btn" title="背景颜色">▧</button>
        </template>
        <div class="mm-rtt-colors">
          <button
            v-for="c in colorList"
            :key="c"
            type="button"
            class="mm-rtt-color"
            :class="{ transparent: c === 'transparent' }"
            :style="c === 'transparent' ? undefined : { background: c }"
            @click="changeBackground(c)"
          />
        </div>
      </NPopover>

      <!-- 对齐 -->
      <NPopover trigger="hover" placement="bottom">
        <template #trigger>
          <button type="button" class="mm-rtt-btn" title="对齐">≡</button>
        </template>
        <div class="mm-rtt-options">
          <div
            v-for="item in alignList"
            :key="item.value"
            class="mm-rtt-option"
            :class="{ active: formatInfo.align === item.value }"
            @click="changeAlign(item.value)"
          >
            {{ item.name }}
          </div>
        </div>
      </NPopover>

      <button type="button" class="mm-rtt-btn" title="清除样式" @click="removeFormat">⌫</button>
    </div>
  </Teleport>
</template>
