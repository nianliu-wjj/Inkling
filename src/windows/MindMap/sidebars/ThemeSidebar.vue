<script setup lang="ts">
import darkList from 'simple-mind-map-plugin-themes/src/dark/index'
import lightList from 'simple-mind-map-plugin-themes/src/light/index'
import themeImgMap from 'simple-mind-map-plugin-themes/themeImgMap'
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { requireMindMap, useMindMap } from '../core/useMindMap'
import { useConfirm } from '../core/confirm'
import SidebarShell from './SidebarShell.vue'

/**
 * 主题侧栏（移植参考端 Theme.vue）。
 *
 * 分组：默认 / 深色 / 浅色（本项目主题包 v1.0.1 的主题项无 dark 字段，改按 dark/light 两个源文件分组）。
 * 切主题前若用户自定义过基础样式（getCustomThemeConfig 非空），弹确认是否覆盖。
 */
const ctx = useMindMap()
const { bus } = ctx
const confirm = useConfirm()

interface ThemeItem {
  name: string
  value: string
}
const groups: { name: string; list: ThemeItem[] }[] = [
  { name: '默认', list: [{ name: '默认', value: 'default' }] },
  { name: '深色', list: darkList.map((t) => ({ name: t.name, value: t.value })) },
  { name: '浅色', list: lightList.map((t) => ({ name: t.name, value: t.value })) },
]

const current = ref('default')

async function use(value: string): Promise<void> {
  const mindMap = requireMindMap(ctx)
  if (value === current.value) return
  // 有自定义基础样式时先确认是否覆盖。
  const custom = mindMap.getCustomThemeConfig()
  if (custom && Object.keys(custom).length > 0) {
    const ok = await confirm('你当前自定义过基础样式，切换主题会覆盖，是否继续？', '切换主题')
    if (!ok) return
    mindMap.setThemeConfig({}, true)
  }
  mindMap.setTheme(value)
  current.value = value
}

const offs: Array<() => void> = []
onMounted(() => {
  current.value = requireMindMap(ctx).getTheme()
  offs.push(bus.on('theme_change', () => (current.value = requireMindMap(ctx).getTheme())))
})
onBeforeUnmount(() => offs.forEach((off) => off()))
</script>

<template>
  <SidebarShell name="theme" title="主题">
    <div v-for="group in groups" :key="group.name" class="mm-theme-group">
      <div class="mm-struct-groupname">{{ group.name }}</div>
      <div class="mm-theme-grid">
        <button
          v-for="item in group.list"
          :key="item.value"
          type="button"
          class="mm-theme-item"
          :class="{ active: item.value === current }"
          :title="item.name"
          @click="use(item.value)"
        >
          <img v-if="themeImgMap[item.value]" :src="themeImgMap[item.value]" :alt="item.name" />
          <span v-else class="mm-theme-noimg">{{ item.name }}</span>
          <span class="mm-theme-name">{{ item.name }}</span>
        </button>
      </div>
    </div>
  </SidebarShell>
</template>
