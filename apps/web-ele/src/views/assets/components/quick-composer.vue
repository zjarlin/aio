<script lang="ts" setup>
import { computed, ref } from 'vue';

import { IconifyIcon } from '@vben/icons';

import { ElButton, ElInput } from 'element-plus';

const props = withDefaults(
  defineProps<{
    placeholder: string;
    submitText?: string;
  }>(),
  {
    submitText: '发送',
  },
);

const emit = defineEmits<{
  submit: [content: string, done: (success?: boolean) => void];
}>();

const content = ref('');
const submitting = ref(false);
const canSubmit = computed(() => content.value.trim().length > 0);

async function submit() {
  const value = content.value.trim();
  if (!value || submitting.value) {
    return;
  }
  submitting.value = true;
  emit('submit', value, (success = true) => {
    submitting.value = false;
    if (success) {
      content.value = '';
    }
  });
}
</script>

<template>
  <section
    class="border-border bg-background overflow-hidden rounded-lg border shadow-sm"
  >
    <ElInput
      v-model="content"
      :autosize="{ minRows: 4, maxRows: 10 }"
      :placeholder="props.placeholder"
      class="quick-composer__input"
      type="textarea"
      @keydown.ctrl.enter.prevent="submit"
      @keydown.meta.enter.prevent="submit"
    />

    <div class="border-border flex items-center justify-end border-t px-3 py-2">
      <ElButton
        :disabled="!canSubmit || submitting"
        :loading="submitting"
        class="quick-composer__send"
        @click="submit"
      >
        <IconifyIcon icon="lucide:send" />
        <span class="sr-only">{{ props.submitText }}</span>
      </ElButton>
    </div>
  </section>
</template>

<style scoped>
.quick-composer__input :deep(.el-textarea__inner) {
  min-height: 116px !important;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas,
    'Liberation Mono', 'Courier New', monospace;
  resize: vertical;
  border: 0;
  border-radius: 0;
  box-shadow: none;
}

.quick-composer__send {
  min-width: 64px;
  color: #fff;
  background: #111827;
  border-color: #111827;
  border-radius: 14px;
}

.quick-composer__send:hover,
.quick-composer__send:focus {
  color: #fff;
  background: #000;
  border-color: #000;
}

.quick-composer__send.is-disabled,
.quick-composer__send.is-disabled:hover,
.quick-composer__send.is-disabled:focus {
  color: #fff;
  background: #111827;
  border-color: #111827;
  opacity: 0.35;
}
</style>
