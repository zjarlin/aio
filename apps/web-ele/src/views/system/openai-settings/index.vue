<script lang="ts" setup>
import { onMounted, reactive, ref } from 'vue';

import { Page } from '@vben/common-ui';

import {
  ElButton,
  ElCard,
  ElForm,
  ElFormItem,
  ElInput,
  ElTag,
} from 'element-plus';

import {
  openAISettingsGetApi,
  type OpenAISettingsRecord,
  openAISettingsSaveApi,
} from '#/api';

const loading = ref(false);
const saving = ref(false);
const current = ref<null | OpenAISettingsRecord>(null);
const form = reactive({
  apiKey: '',
  baseUrl: '',
  model: '',
});

async function loadSettings() {
  loading.value = true;
  try {
    const result = await openAISettingsGetApi();
    current.value = result;
    Object.assign(form, {
      apiKey: '',
      baseUrl: result.baseUrl,
      model: result.model,
    });
  } finally {
    loading.value = false;
  }
}

async function saveSettings() {
  saving.value = true;
  try {
    const result = await openAISettingsSaveApi({
      apiKey: form.apiKey.trim() ? form.apiKey : undefined,
      baseUrl: form.baseUrl,
      model: form.model,
    });
    current.value = result;
    form.apiKey = '';
  } finally {
    saving.value = false;
  }
}

async function clearApiKey() {
  saving.value = true;
  try {
    const result = await openAISettingsSaveApi({
      apiKey: '',
    });
    current.value = result;
    form.apiKey = '';
  } finally {
    saving.value = false;
  }
}

onMounted(loadSettings);
</script>

<template>
  <Page
    description="本地保存 OpenAI 访问配置，供助手和知识库调用使用"
    title="OpenAI 设置"
  >
    <div class="grid gap-4 xl:grid-cols-[420px_minmax(0,1fr)]">
      <ElCard shadow="never">
        <template #header>
          <div class="flex items-center justify-between">
            <span class="font-medium">当前状态</span>
            <ElTag :type="current?.apiKeyConfigured ? 'success' : 'warning'">
              {{ current?.apiKeyConfigured ? '已配置' : '未配置' }}
            </ElTag>
          </div>
        </template>

        <div class="space-y-3 text-sm">
          <div class="flex items-center justify-between gap-3">
            <span class="text-muted-foreground">API Key</span>
            <span class="font-mono text-xs">
              {{ current?.apiKeyPreview || '未设置' }}
            </span>
          </div>
          <div class="flex items-center justify-between gap-3">
            <span class="text-muted-foreground">Base URL</span>
            <span class="break-all font-mono text-xs">
              {{ current?.baseUrl || '默认' }}
            </span>
          </div>
          <div class="flex items-center justify-between gap-3">
            <span class="text-muted-foreground">Model</span>
            <span class="font-mono text-xs">
              {{ current?.model || 'gpt-5.5' }}
            </span>
          </div>
        </div>
      </ElCard>

      <ElCard shadow="never">
        <template #header>
          <div class="font-medium">配置</div>
        </template>

        <ElForm :model="form" label-width="110px">
          <ElFormItem label="API Key">
            <ElInput
              v-model="form.apiKey"
              placeholder="留空表示保持当前 Key 不变"
              show-password
              type="password"
            />
          </ElFormItem>
          <ElFormItem label="Base URL">
            <ElInput
              v-model="form.baseUrl"
              placeholder="例如 https://api.addzero.site"
            />
          </ElFormItem>
          <ElFormItem label="Model">
            <ElInput v-model="form.model" placeholder="默认 gpt-5.5" />
          </ElFormItem>
          <ElFormItem>
            <div class="flex items-center gap-2">
              <ElButton :loading="saving" type="primary" @click="saveSettings">
                保存
              </ElButton>
              <ElButton
                :disabled="!current?.apiKeyConfigured"
                :loading="saving"
                @click="clearApiKey"
              >
                清空 Key
              </ElButton>
              <ElButton :loading="loading" @click="loadSettings">刷新</ElButton>
            </div>
          </ElFormItem>
        </ElForm>
      </ElCard>
    </div>
  </Page>
</template>
