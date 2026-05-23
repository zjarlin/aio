<script lang="ts" setup>
import AssetItemManager from '../components/asset-item-manager.vue';
</script>

<template>
  <AssetItemManager
    default-category="env"
    description="维护本机环境变量、变量说明和值模板"
    kind="env_vars"
    quick-placeholder="写下环境变量定义，首行会作为名称；支持 Markdown 和 #标签"
    quick-submit-text="保存变量"
    title="环境变量管理"
  />
</template>
