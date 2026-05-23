<script lang="ts" setup>
import AssetItemManager from '../components/asset-item-manager.vue';
</script>

<template>
  <AssetItemManager
    default-category="terminal"
    description="维护本机 CLI 命令、别名和常用工具说明"
    kind="cli"
    quick-placeholder="写下 CLI 命令和使用场景，首行会作为名称；支持 Markdown 和 #标签"
    quick-submit-text="保存命令"
    title="CLI 管理"
  />
</template>
