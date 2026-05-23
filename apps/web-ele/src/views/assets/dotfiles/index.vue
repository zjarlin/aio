<script lang="ts" setup>
import AssetItemManager from '../components/asset-item-manager.vue';
</script>

<template>
  <AssetItemManager
    default-category="dotfiles"
    description="维护 dotfiles 条目、说明和片段内容，不直接写入真实文件"
    kind="dotfiles"
    quick-placeholder="写下 dotfiles 内容，首行会作为名称；支持 Markdown 和 #标签"
    quick-submit-text="保存条目"
    title="dotfiles 管理"
  />
</template>
