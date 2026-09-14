<script setup lang="ts">
import { shortcutKeyList } from '../constants/shortcuts'

/** 快捷键说明侧栏：分组展示 constants/shortcuts.ts；实际绑定由库的 KeyboardNavigation 完成。 */
</script>

<template>
  <div v-for="group in shortcutKeyList" :key="group.type" class="mm-shortcut-group">
    <div class="mm-shortcut-type">{{ group.type }}</div>
    <div v-for="item in group.list" :key="item.name" class="mm-shortcut-row">
      <span class="mm-shortcut-name">{{ item.name }}</span>
      <kbd class="mm-shortcut-key">{{ item.value }}</kbd>
    </div>
  </div>
</template>
