<script lang="ts" setup>
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';

import * as monaco from 'monaco-editor';
import EditorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';

const props = withDefaults(
  defineProps<{
    height?: string;
    modifiedValue: string;
    originalValue: string;
  }>(),
  {
    height: '520px',
  },
);

const container = ref<HTMLElement>();
let diffEditor: monaco.editor.IStandaloneDiffEditor | undefined;
let originalModel: monaco.editor.ITextModel | undefined;
let modifiedModel: monaco.editor.ITextModel | undefined;

(
  globalThis as {
    MonacoEnvironment?: monaco.Environment;
  } & typeof globalThis
).MonacoEnvironment = {
  getWorker() {
    return new EditorWorker();
  },
};

onMounted(() => {
  if (!container.value) {
    return;
  }

  diffEditor = monaco.editor.createDiffEditor(container.value, {
    automaticLayout: true,
    fontSize: 13,
    minimap: { enabled: false },
    originalEditable: false,
    readOnly: true,
    renderSideBySide: true,
    renderWhitespace: 'selection',
    scrollBeyondLastLine: false,
    theme: 'vs',
    wordWrap: 'on',
  });

  originalModel = monaco.editor.createModel(props.originalValue, 'yaml');
  modifiedModel = monaco.editor.createModel(props.modifiedValue, 'yaml');
  diffEditor.setModel({
    original: originalModel,
    modified: modifiedModel,
  });
});

watch(
  () => [props.originalValue, props.modifiedValue] as const,
  ([originalValue, modifiedValue]) => {
    if (!originalModel || !modifiedModel) {
      return;
    }
    const nextOriginalValue = originalValue ?? '';
    const nextModifiedValue = modifiedValue ?? '';
    if (originalModel.getValue() !== nextOriginalValue) {
      originalModel.setValue(nextOriginalValue);
    }
    if (modifiedModel.getValue() !== nextModifiedValue) {
      modifiedModel.setValue(nextModifiedValue);
    }
  },
);

onBeforeUnmount(() => {
  diffEditor?.dispose();
  originalModel?.dispose();
  modifiedModel?.dispose();
  diffEditor = undefined;
  originalModel = undefined;
  modifiedModel = undefined;
});
</script>

<template>
  <div
    ref="container"
    :style="{ height }"
    class="overflow-hidden rounded border"
  ></div>
</template>
