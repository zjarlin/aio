<script lang="ts" setup>
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';

import * as monaco from 'monaco-editor';
import EditorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';

const props = withDefaults(
  defineProps<{
    height?: string;
    modelValue: string;
  }>(),
  {
    height: '560px',
  },
);

const emit = defineEmits<{
  selectionChange: [value: string];
  'update:modelValue': [value: string];
}>();

const container = ref<HTMLElement>();
let editor: monaco.editor.IStandaloneCodeEditor | undefined;
let applyingExternalValue = false;

(
  globalThis as {
    MonacoEnvironment?: monaco.Environment;
  } & typeof globalThis
).MonacoEnvironment = {
  getWorker() {
    return new EditorWorker();
  },
};

function getSelectionText() {
  if (!editor) {
    return '';
  }
  const model = editor.getModel();
  const selection = editor.getSelection();
  if (!model || !selection) {
    return '';
  }
  return model.getValueInRange(selection);
}

function replaceSelection(value: string) {
  if (!editor) {
    return;
  }
  const selection = editor.getSelection();
  if (!selection) {
    return;
  }
  editor.executeEdits('extract-variable', [{ range: selection, text: value }]);
  editor.focus();
}

onMounted(() => {
  if (!container.value) {
    return;
  }

  editor = monaco.editor.create(container.value, {
    automaticLayout: true,
    fontSize: 13,
    language: 'yaml',
    minimap: { enabled: false },
    renderWhitespace: 'selection',
    scrollBeyondLastLine: false,
    tabSize: 2,
    theme: 'vs',
    value: props.modelValue,
    wordWrap: 'on',
  });

  editor.onDidChangeModelContent(() => {
    if (!editor || applyingExternalValue) {
      return;
    }
    emit('update:modelValue', editor.getValue());
  });

  editor.onDidChangeCursorSelection(() => {
    emit('selectionChange', getSelectionText());
  });
});

watch(
  () => props.modelValue,
  (value) => {
    if (!editor || value === editor.getValue()) {
      return;
    }
    applyingExternalValue = true;
    editor.setValue(value);
    applyingExternalValue = false;
  },
);

onBeforeUnmount(() => {
  editor?.dispose();
  editor = undefined;
});

defineExpose({
  getSelectionText,
  replaceSelection,
});
</script>

<template>
  <div
    ref="container"
    :style="{ height }"
    class="overflow-hidden rounded border"
  ></div>
</template>
