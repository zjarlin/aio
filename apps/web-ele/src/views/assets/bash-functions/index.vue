<script lang="ts" setup>
import AssetItemManager from '../components/asset-item-manager.vue';
</script>

<template>
  <AssetItemManager
    default-category="shell"
    description="维护 Bash 函数定义、注释和常用脚本片段"
    kind="bash_functions"
    quick-placeholder="写下 Bash 函数内容，首行会作为名称；支持 Markdown 和 #标签"
    quick-submit-text="保存函数"
    title="Bash 函数管理"
  />
</template>
