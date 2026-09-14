<script setup lang="ts">
import { onBeforeUnmount, onMounted, watch } from 'vue'
import { logger } from '@/service/logger'

/**
 * 通用节点弹窗壳（原型 `#mmModalOverlay` + `.mm-modal`，`docs/index.html:397-410`）。
 *
 * 原型用一个遮罩复用 8 种表单；我们此前每个表单各自成组件（差异大，保留），
 * 只有图标与公式是从右侧抽屉改过来的（spec D42），故这里只抽这两个共用的壳。
 *
 * 样式一律用生成层的 `.mm-modal*`（spec D45），本组件不写私有样式，免得与 `--mm-*` 令牌体系脱钩。
 *
 * @author nianliu-jj
 * @since 2026-09-14
 */
const props = defineProps<{ title: string; visible: boolean }>()
const emit = defineEmits<{ close: [] }>()

function close(): void {
  emit('close')
}

/**
 * Esc 关闭。
 * 原型把通用弹窗列为 Esc 的首要处理对象（`docs/app.js:530`），本窗口已有同款写法
 * （`popups/NodeImgPreview.vue:31` 的 `Escape && show`），这里沿用：仅在自身可见时响应。
 */
function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape' && props.visible) close()
}

onMounted(() => window.addEventListener('keydown', onKeydown))
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))

/**
 * 弹窗开合是本窗口里少见的状态切换，记一条日志便于排查「点不动 / 关不掉」类问题
 * （照 `sidebars/MmDrawer.vue` 对抽屉开合的做法）。打标题而不是组件名：标题才是用户看到的那个名字。
 */
watch(
  () => props.visible,
  (value) => logger.info('mindmap', `节点弹窗「${props.title}」：${value ? '打开' : '关闭'}`),
)
</script>

<template>
  <!-- `@click.self` 只认遮罩自身：弹窗内部的点击不冒泡到这里，免得点输入框顺手把窗关了 -->
  <div v-if="visible" class="mm-modal-overlay" @click.self="close">
    <div class="mm-modal">
      <div class="mm-modal-header">
        <span>{{ title }}</span>
        <button type="button" class="mm-modal-close" title="关闭" @click="close">✕</button>
      </div>
      <div class="mm-modal-body"><slot /></div>
      <!-- 没有 footer 插槽时整条不渲染：生成层 `.mm-modal-footer` 带边框与底色，空着会留一条空带 -->
      <div v-if="$slots.footer" class="mm-modal-footer"><slot name="footer" /></div>
    </div>
  </div>
</template>
