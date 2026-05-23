<script lang="ts" setup>
import { computed, reactive, ref } from 'vue';

import { Page } from '@vben/common-ui';
import { IconifyIcon } from '@vben/icons';

import { ElButton, ElInput, ElTabPane, ElTabs, ElTag } from 'element-plus';

import {
  openAIAssistantChatApi,
  type OpenAIAssistantChatResponse,
  type OpenAIAssistantPageContextInput,
  type OpenAIAssistantPageContextPreview,
  openAIAssistantPreviewContextApi,
  type OpenAIAssistantRole,
  type OpenAIAssistantTurn,
} from '#/api';

type ContextMode = 'html' | 'text' | 'url';

const contextMode = ref<ContextMode>('url');
const previewLoading = ref(false);
const sending = ref(false);
const question = ref('');
const conversation = ref<OpenAIAssistantTurn[]>([]);
const contextPreview = ref<null | OpenAIAssistantPageContextPreview>(null);
const context = reactive({
  html: '',
  selection: '',
  text: '',
  title: '',
  url: '',
});

const canSend = computed(
  () => question.value.trim().length > 0 && !sending.value,
);
const previewText = computed(
  () => contextPreview.value?.content ?? '尚未生成上下文。',
);
const previewTitle = computed(
  () => contextPreview.value?.title ?? '上下文预览',
);

function buildContextInput(): OpenAIAssistantPageContextInput {
  const input: OpenAIAssistantPageContextInput = {
    selection: context.selection.trim() || undefined,
    title: context.title.trim() || undefined,
  };

  if (contextMode.value === 'url') {
    input.url = context.url.trim() || undefined;
  }
  if (contextMode.value === 'html') {
    input.html = context.html.trim() || undefined;
  }
  if (contextMode.value === 'text') {
    input.text = context.text.trim() || undefined;
  }

  return input;
}

async function previewContext() {
  previewLoading.value = true;
  try {
    contextPreview.value =
      await openAIAssistantPreviewContextApi(buildContextInput());
  } finally {
    previewLoading.value = false;
  }
}

function clearContext() {
  contextMode.value = 'url';
  Object.assign(context, {
    html: '',
    selection: '',
    text: '',
    title: '',
    url: '',
  });
  contextPreview.value = null;
}

function clearConversation() {
  conversation.value = [];
  question.value = '';
}

function pushTurn(role: OpenAIAssistantRole, content: string) {
  conversation.value.push({ content, role });
}

async function sendQuestion() {
  const value = question.value.trim();
  if (!value || sending.value) {
    return;
  }

  const history = conversation.value.slice(-12);
  pushTurn('user', value);
  question.value = '';
  sending.value = true;

  try {
    const result: OpenAIAssistantChatResponse = await openAIAssistantChatApi({
      context: buildContextInput(),
      history,
      question: value,
    });
    contextPreview.value = result.context;
    pushTurn('assistant', result.answer);
  } catch (error) {
    conversation.value.pop();
    question.value = value;
    throw error;
  } finally {
    sending.value = false;
  }
}
</script>

<template>
  <Page description="OpenAI 页面上下文助手" title="OpenAI 助手">
    <div class="grid gap-4 xl:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
      <section
        class="border-border bg-background flex min-h-[720px] flex-col rounded-lg border shadow-sm"
      >
        <div
          class="border-border flex items-center justify-between border-b px-4 py-3"
        >
          <div>
            <div class="text-sm font-medium">页面上下文</div>
            <div class="text-muted-foreground text-xs">URL、HTML、文本</div>
          </div>
          <div class="flex items-center gap-2">
            <ElButton :loading="previewLoading" @click="previewContext">
              <IconifyIcon icon="lucide:refresh-cw" />
              <span>预览</span>
            </ElButton>
            <ElButton @click="clearContext">
              <IconifyIcon icon="lucide:trash-2" />
              <span>清空</span>
            </ElButton>
          </div>
        </div>

        <div class="flex-1 space-y-4 p-4">
          <ElTabs v-model="contextMode">
            <ElTabPane label="URL" name="url">
              <div class="space-y-3">
                <ElInput v-model="context.title" placeholder="标题" />
                <ElInput v-model="context.url" placeholder="页面 URL" />
              </div>
            </ElTabPane>
            <ElTabPane label="HTML" name="html">
              <div class="space-y-3">
                <ElInput v-model="context.title" placeholder="标题" />
                <ElInput
                  v-model="context.html"
                  :rows="12"
                  placeholder="粘贴 HTML"
                  type="textarea"
                />
              </div>
            </ElTabPane>
            <ElTabPane label="文本" name="text">
              <div class="space-y-3">
                <ElInput v-model="context.title" placeholder="标题" />
                <ElInput
                  v-model="context.text"
                  :rows="12"
                  placeholder="粘贴页面正文"
                  type="textarea"
                />
              </div>
            </ElTabPane>
          </ElTabs>

          <ElInput
            v-model="context.selection"
            :rows="5"
            placeholder="选中内容"
            type="textarea"
          />

          <div class="space-y-2">
            <div class="flex flex-wrap items-center gap-2">
              <ElTag v-if="contextPreview" size="small" type="success">
                {{ contextPreview.source }}
              </ElTag>
              <ElTag
                v-if="contextPreview?.truncated"
                size="small"
                type="warning"
              >
                截断
              </ElTag>
              <ElTag v-if="contextPreview" size="small">
                {{ contextPreview.characterCount }} 字符
              </ElTag>
            </div>
            <div class="text-muted-foreground text-xs">
              {{ previewTitle }}
            </div>
            <ElInput
              :model-value="previewText"
              :rows="16"
              readonly
              type="textarea"
            />
          </div>
        </div>
      </section>

      <section
        class="border-border bg-background flex min-h-[720px] flex-col rounded-lg border shadow-sm"
      >
        <div
          class="border-border flex items-center justify-between border-b px-4 py-3"
        >
          <div>
            <div class="text-sm font-medium">对话</div>
            <div class="text-muted-foreground text-xs">保留最近的问答历史</div>
          </div>
          <ElButton
            :disabled="conversation.length === 0"
            @click="clearConversation"
          >
            <IconifyIcon icon="lucide:trash-2" />
            <span>清空</span>
          </ElButton>
        </div>

        <div class="min-h-0 flex-1 space-y-3 overflow-auto p-4">
          <article
            v-for="(item, index) in conversation"
            :key="`${item.role}-${index}`"
            :class="[
              item.role === 'user' ? 'bg-muted ml-10' : 'bg-background mr-10',
            ]"
            class="border-border rounded-lg border px-3 py-2"
          >
            <div class="mb-2 flex items-center justify-between gap-2">
              <ElTag
                :type="item.role === 'user' ? 'warning' : 'success'"
                size="small"
              >
                {{ item.role === 'user' ? '用户' : '助手' }}
              </ElTag>
            </div>
            <div class="whitespace-pre-wrap text-sm leading-6">
              {{ item.content }}
            </div>
          </article>
        </div>

        <div class="border-border border-t p-4">
          <ElInput
            v-model="question"
            :disabled="sending"
            :rows="5"
            placeholder="输入问题，Ctrl+Enter 发送"
            type="textarea"
            @keydown.ctrl.enter.prevent="sendQuestion"
            @keydown.meta.enter.prevent="sendQuestion"
          />
          <div
            class="border-border flex items-center justify-end gap-2 border-t px-3 py-2"
          >
            <ElButton
              :disabled="!canSend"
              :loading="sending"
              type="primary"
              @click="sendQuestion"
            >
              <IconifyIcon icon="lucide:send" />
              <span>发送</span>
            </ElButton>
          </div>
        </div>
      </section>
    </div>
  </Page>
</template>
