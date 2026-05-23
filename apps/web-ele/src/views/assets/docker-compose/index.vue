<script lang="ts" setup>
import type { FormInstance, FormRules } from 'element-plus';

import { computed, nextTick, onMounted, reactive, ref } from 'vue';

import { Page } from '@vben/common-ui';

import {
  ElButton,
  ElDrawer,
  ElEmpty,
  ElForm,
  ElFormItem,
  ElInput,
  ElMessage,
  ElMessageBox,
  ElOption,
  ElPagination,
  ElSelect,
  ElTag,
} from 'element-plus';

import {
  assetItemCreateApi,
  assetItemDeleteApi,
  type AssetItemDeployPreview,
  assetItemDeployPreviewApi,
  assetItemDeploySaveApi,
  assetItemImportDirectoryApi,
  assetItemPageApi,
  type AssetItemRecord,
  assetItemToggleApi,
  assetItemUpdateApi,
  type AssetVariableCandidate,
  assetVariableDeleteApi,
  assetVariablePageApi,
  type AssetVariableRecord,
  assetVariableRefreshPageGlobalsApi,
  assetVariableUpsertApi,
} from '#/api';

import { formatTime } from '../../system/shared';
import MonacoYamlDiffEditor from '../components/monaco-yaml-diff-editor.vue';
import MonacoYamlEditor from '../components/monaco-yaml-editor.vue';
import {
  assetUsageOptions,
  assetUsageText,
  displayTags,
  nextAssetUsageStatus,
  nextAssetUsageText,
  normalizeTags,
} from '../shared';

const defaultDeployRoot = '~/DockerCompose';
const defaultRootPath = defaultDeployRoot;
const pageGlobalVariableCategory = 'docker_compose_common';

const loading = ref(false);
const syncing = ref(false);
const refreshingGlobals = ref(false);
const deploying = ref(false);
const items = ref<AssetItemRecord[]>([]);
const keyword = ref('');
const category = ref<string[]>([]);
const status = ref('');
const rootPath = ref(defaultRootPath);
const editorVisible = ref(false);
const variableVisible = ref(false);
const deployVisible = ref(false);
const selectedText = ref('');
const editorRef = ref<InstanceType<typeof MonacoYamlEditor>>();
const editingId = ref('');
const composeFormRef = ref<FormInstance>();
const extractionScope = ref<'file' | 'grid'>('file');
const variableScope = ref<'file' | 'grid'>('grid');
const gridVariables = ref<AssetVariableRecord[]>([]);
const fileVariables = ref<AssetVariableRecord[]>([]);
const variableDrawerOrigin = ref<'editor' | 'standalone' | null>(null);
const page = reactive({ o: 0, s: 12, total: 0 });
const deployPreview = ref<AssetItemDeployPreview>();
const deployMergedContent = ref('');
const form = reactive({
  category: '',
  code: '',
  content: '',
  contentHash: '',
  description: '',
  fileName: '',
  images: [] as string[],
  lastSyncedAt: 0,
  name: '',
  ports: [] as string[],
  serviceCount: 0,
  services: [] as string[],
  sortOrder: 0,
  sourceMtime: 0,
  sourcePath: '',
  sourceSize: 0,
  status: 'disabled',
  tags: [] as string[],
  validationIssues: [] as AssetItemRecord['validationIssues'],
  validationStatus: 'unknown' as AssetItemRecord['validationStatus'],
  variableCandidates: [] as AssetVariableCandidate[],
  volumes: [] as string[],
});
const composeFormRules: FormRules = {
  name: [{ message: '名称必填', required: true, trigger: 'blur' }],
};
const variableForm = reactive({
  category: '',
  defaultValue: '',
  description: '',
  id: '',
  key: '',
  sortOrder: 0,
  status: 'enabled',
  value: '',
  valueKind: 'text',
});

const dialogTitle = computed(() =>
  editingId.value ? form.name : '新增 Compose',
);
const variableTitle = computed(() =>
  variableScope.value === 'grid' ? '页面级全局变量' : '当前 YAML 变量',
);
const variableRows = computed(() =>
  variableScope.value === 'grid' ? gridVariables.value : fileVariables.value,
);
const categories = computed(() => {
  const values = items.value.flatMap((item) =>
    displayTags(item.category, item.tags),
  );
  return [...new Set(values)];
});

async function loadItems() {
  loading.value = true;
  try {
    const result = await assetItemPageApi({
      categories: category.value.length > 0 ? category.value : undefined,
      kind: 'docker_compose',
      keyword: keyword.value,
      o: page.o,
      s: page.s,
      status: status.value || undefined,
    });
    items.value = result.d;
    page.total = result.t;
    await loadGridVariables();
  } finally {
    loading.value = false;
  }
}

async function loadGridVariables() {
  const result = await assetVariablePageApi({
    category: pageGlobalVariableCategory,
    kind: 'docker_compose',
    o: 0,
    s: 200,
    scope: 'grid',
  });
  gridVariables.value = result.d;
}

async function refreshPageGlobalVariables() {
  refreshingGlobals.value = true;
  try {
    const result = await assetVariableRefreshPageGlobalsApi();
    await loadGridVariables();
    ElMessage.success(
      `已扫描 ${result.scanned} 个 Compose，内置 ${result.candidates} 个公共变量，新增 ${result.inserted}，更新 ${result.updated}，保留 ${result.protected}`,
    );
  } finally {
    refreshingGlobals.value = false;
  }
}

async function loadFileVariables(assetItemId = editingId.value) {
  if (!assetItemId) {
    fileVariables.value = [];
    return;
  }
  const result = await assetVariablePageApi({
    assetItemId,
    kind: 'docker_compose',
    o: 0,
    s: 200,
    scope: 'file',
  });
  fileVariables.value = result.d;
}

async function prepareVariableDrawer() {
  if (variableVisible.value) {
    return;
  }

  variableDrawerOrigin.value = editorVisible.value ? 'editor' : 'standalone';
  if (editorVisible.value) {
    editorVisible.value = false;
    await nextTick();
  }
}

async function importDirectory() {
  syncing.value = true;
  try {
    const result = await assetItemImportDirectoryApi({
      kind: 'docker_compose',
      rootPath: rootPath.value,
    });
    ElMessage.success(
      `扫描 ${result.scanned} 个 YAML，新增 ${result.imported}，更新 ${result.updated}，未变 ${result.unchanged}，跳过 ${result.skipped}`,
    );
    page.o = 0;
    await loadItems();
  } finally {
    syncing.value = false;
  }
}

async function deployItem(row: AssetItemRecord) {
  deploying.value = true;
  try {
    const preview = await assetItemDeployPreviewApi({
      id: row.id,
      rootPath: rootPath.value,
    });
    if (preview.hasConflict) {
      deployPreview.value = preview;
      deployMergedContent.value = preview.libraryContent;
      deployVisible.value = true;
      return;
    }

    const saved = await assetItemDeploySaveApi({
      content: preview.libraryContent,
      id: preview.id,
      rootPath: rootPath.value,
    });
    ElMessage.success(`已部署到 ${preview.targetRelativePath}`);
    upsertLoadedItem(saved);
  } finally {
    deploying.value = false;
  }
}

async function saveDeployMerge() {
  if (!deployPreview.value) {
    return;
  }
  deploying.value = true;
  try {
    const saved = await assetItemDeploySaveApi({
      content: deployMergedContent.value,
      id: deployPreview.value.id,
      rootPath: rootPath.value,
    });
    ElMessage.success(`已部署到 ${deployPreview.value.targetRelativePath}`);
    deployVisible.value = false;
    deployPreview.value = undefined;
    deployMergedContent.value = '';
    upsertLoadedItem(saved);
  } finally {
    deploying.value = false;
  }
}

function upsertLoadedItem(record: AssetItemRecord) {
  const index = items.value.findIndex((item) => item.id === record.id);
  if (index !== -1) {
    items.value.splice(index, 1, record);
  }
}

function currentEditingRecord() {
  return items.value.find((item) => item.id === editingId.value);
}

function useLibraryDeployContent() {
  deployMergedContent.value = deployPreview.value?.libraryContent || '';
}

function useLocalDeployContent() {
  deployMergedContent.value = deployPreview.value?.localContent || '';
}

function resetForm() {
  Object.assign(form, {
    category: '',
    code: '',
    content: 'services:\n  app:\n    image: nginx:latest\n',
    contentHash: '',
    description: '',
    fileName: '',
    images: [],
    lastSyncedAt: 0,
    name: '',
    ports: [],
    serviceCount: 0,
    services: [],
    sortOrder: 0,
    sourceMtime: 0,
    sourcePath: '',
    sourceSize: 0,
    status: 'disabled',
    tags: [],
    validationIssues: [],
    validationStatus: 'unknown',
    variableCandidates: [],
    volumes: [],
  });
}

function slugify(value: string) {
  const slug = value
    .trim()
    .toLowerCase()
    .replaceAll(/[^a-z0-9]+/g, '-')
    .replaceAll(/^-+|-+$/g, '');
  return `asset-docker-compose-${slug || 'compose'}-${Date.now().toString(36)}`;
}

function openCreate() {
  editingId.value = '';
  resetForm();
  editorVisible.value = true;
}

function openEdit(row: AssetItemRecord) {
  editingId.value = row.id;
  Object.assign(form, {
    category: row.category,
    code: row.code,
    content: row.content,
    contentHash: row.contentHash,
    description: row.description,
    fileName: row.fileName,
    images: [...row.images],
    lastSyncedAt: row.lastSyncedAt,
    name: row.name,
    ports: [...row.ports],
    serviceCount: row.serviceCount,
    services: [...row.services],
    sortOrder: row.sortOrder,
    sourceMtime: row.sourceMtime,
    sourcePath: row.sourcePath,
    sourceSize: row.sourceSize,
    status: row.status,
    tags: displayTags(row.category, row.tags),
    validationIssues: [...row.validationIssues],
    validationStatus: row.validationStatus,
    variableCandidates: [...row.variableCandidates],
    volumes: [...row.volumes],
  });
  selectedText.value = '';
  void loadFileVariables(row.id);
  editorVisible.value = true;
}

async function saveItem(closeEditor = true) {
  await composeFormRef.value?.validate();
  const tags = normalizeTags(form.tags);
  const name = form.name.trim();
  const input = {
    ...form,
    category: tags[0] || '',
    code: form.code.trim() || slugify(name || form.fileName || 'compose'),
    kind: 'docker_compose' as const,
    name: name || form.fileName || 'Compose',
    status: form.status,
    tags,
  };

  const saved = await (editingId.value
    ? assetItemUpdateApi({ id: editingId.value, ...input })
    : assetItemCreateApi(input));

  editingId.value = saved.id;
  if (closeEditor) {
    editorVisible.value = false;
  }
  await loadItems();
  return saved;
}

async function deployEditingItem() {
  let row = currentEditingRecord();
  if (!editingId.value || !row) {
    ElMessage.warning('先保存到库再部署');
    return;
  }

  if (
    form.content !== row.content ||
    form.name.trim() !== row.name ||
    normalizeTags(form.tags).join('\n') !==
      displayTags(row.category, row.tags).join('\n')
  ) {
    try {
      await ElMessageBox.confirm(
        '当前编辑内容未保存，是否先保存到库再部署？',
        '部署确认',
        { confirmButtonText: '保存并部署' },
      );
    } catch {
      return;
    }
    row = await saveItem(false);
  }

  editorVisible.value = false;
  await deployItem(row);
}

async function toggleItem(row: AssetItemRecord) {
  const nextStatus = nextAssetUsageStatus(row.status);
  await assetItemToggleApi(row.id, nextStatus);
  await loadItems();
}

async function deleteItem(row: AssetItemRecord) {
  await ElMessageBox.confirm(`确认删除 ${row.name}？`, '删除确认');
  await assetItemDeleteApi(row.id);
  await loadItems();
}

async function extractSelectionAsVariable() {
  const selected =
    editorRef.value?.getSelectionText().trim() || selectedText.value.trim();
  if (!selected) {
    ElMessage.warning('先在编辑器里选中要提取的字符串');
    return;
  }

  const { value } = await ElMessageBox.prompt('变量名', '提取变量', {
    confirmButtonText: '提取',
    inputPattern: /^[A-Z][A-Z0-9_]*$/,
    inputValue: guessVariableKey(selected),
    inputErrorMessage: '变量名需使用大写字母、数字和下划线，并以字母开头',
  });
  const key = sanitizeVariableKey(value || 'VALUE');
  editorRef.value?.replaceSelection(`\${${key}}`);
  form.variableCandidates = mergeCandidates([
    ...form.variableCandidates,
    {
      key,
      kind: 'manual',
      occurrences: 1,
      scope: extractionScope.value,
      source: 'selection',
      value: selected,
    },
  ]);
  await persistExtractedVariable(key, selected);
  selectedText.value = '';
}

async function persistExtractedVariable(key: string, value: string) {
  const valueKind = inferVariableKind(value, key);
  const currentTag =
    displayTags(form.category, form.tags)[0] || category.value[0] || '';
  if (extractionScope.value === 'grid') {
    await assetVariableUpsertApi({
      category: currentTag,
      defaultValue: value,
      key,
      kind: 'docker_compose',
      source: 'selection',
      value,
      valueKind,
    });
    await loadGridVariables();
    return;
  }

  if (!editingId.value) {
    ElMessage.info('新建 YAML 保存后可写入文件级变量库');
    return;
  }

  await assetVariableUpsertApi({
    assetItemId: editingId.value,
    category: currentTag,
    defaultValue: value,
    key,
    kind: 'docker_compose',
    source: 'selection',
    value,
    valueKind,
  });
  await loadFileVariables();
}

function currentGlobalCategory() {
  return pageGlobalVariableCategory;
}

function candidateValueKind(candidate: AssetVariableCandidate) {
  const kind = candidate.kind.trim().toLowerCase();
  if (['image', 'path', 'port', 'secret', 'text', 'url'].includes(kind)) {
    return kind;
  }
  return inferVariableKind(candidate.value, candidate.key);
}

async function findGlobalVariable(variableCategory: string, key: string) {
  const normalizedKey = sanitizeVariableKey(key);
  const cached = gridVariables.value.find(
    (row) => row.category === variableCategory && row.key === normalizedKey,
  );
  if (cached) {
    return cached;
  }

  const result = await assetVariablePageApi({
    category: variableCategory || undefined,
    kind: 'docker_compose',
    keyword: normalizedKey,
    o: 0,
    s: 50,
    scope: 'grid',
  });
  return result.d.find(
    (row) => row.category === variableCategory && row.key === normalizedKey,
  );
}

async function promoteCandidate(candidate: AssetVariableCandidate) {
  const key = sanitizeVariableKey(candidate.key);
  const variableCategory = currentGlobalCategory();
  const existing = await findGlobalVariable(variableCategory, key);
  if (existing) {
    ElMessage.info('页面级全局变量已存在');
    return;
  }

  await assetVariableUpsertApi({
    category: variableCategory,
    defaultValue: candidate.value,
    description: `从 ${form.name || form.fileName || 'Compose YAML'} 提升，出现 ${
      candidate.occurrences || 1
    } 次`,
    key,
    kind: 'docker_compose',
    source: candidate.source === 'selection' ? 'selection' : 'manual',
    status: 'enabled',
    value: candidate.value,
    valueKind: candidateValueKind(candidate),
  });
  await loadGridVariables();
  ElMessage.success('已提升为页面级全局变量');
}

async function promoteVariable(row: AssetVariableRecord) {
  if (row.scope === 'grid') {
    ElMessage.info('页面级全局变量不可直接编辑');
    return;
  }

  const key = sanitizeVariableKey(row.key);
  const variableCategory = row.category || currentGlobalCategory();
  const existing = await findGlobalVariable(variableCategory, key);
  if (existing) {
    ElMessage.info('页面级全局变量已存在');
    return;
  }

  await assetVariableUpsertApi({
    category: variableCategory,
    defaultValue: row.defaultValue || row.value,
    description: row.description,
    key,
    kind: 'docker_compose',
    sortOrder: row.sortOrder,
    source: row.source === 'selection' ? 'selection' : 'manual',
    status: row.status,
    value: row.value || row.defaultValue,
    valueKind:
      row.valueKind ||
      inferVariableKind(row.value || row.defaultValue, row.key),
  });
  await loadGridVariables();
  ElMessage.success('已提升为页面级全局变量');
}

function resetVariableForm() {
  Object.assign(variableForm, {
    category:
      variableScope.value === 'grid'
        ? currentGlobalCategory()
        : displayTags(form.category, form.tags)[0] || '',
    defaultValue: '',
    description: '',
    id: '',
    key: '',
    sortOrder: 0,
    status: 'enabled',
    value: '',
    valueKind: 'text',
  });
}

async function openVariableManager(scope: 'file' | 'grid') {
  await prepareVariableDrawer();
  variableScope.value = scope;
  resetVariableForm();
  variableVisible.value = true;
  await (scope === 'grid' ? loadGridVariables() : loadFileVariables());
}

function handleVariableDrawerClosed() {
  if (variableDrawerOrigin.value === 'editor') {
    editorVisible.value = true;
  }
  variableDrawerOrigin.value = null;
}

async function saveVariable() {
  if (variableScope.value === 'file' && !editingId.value) {
    ElMessage.warning('先保存 YAML，再设置文件级变量');
    return;
  }

  const key = variableForm.key.trim();
  const value = variableForm.value.trim();
  if (!key) {
    ElMessage.warning('变量名不能为空');
    return;
  }
  if (!value) {
    ElMessage.warning('当前值不能为空');
    return;
  }

  await assetVariableUpsertApi({
    assetItemId: variableScope.value === 'file' ? editingId.value : undefined,
    category:
      variableScope.value === 'grid'
        ? pageGlobalVariableCategory
        : displayTags(form.category, form.tags)[0] || variableForm.category,
    defaultValue: value,
    description: variableForm.description,
    id: variableForm.id || undefined,
    key: sanitizeVariableKey(key),
    kind: 'docker_compose',
    sortOrder: variableForm.sortOrder,
    source: 'manual',
    status: variableForm.status,
    value,
    valueKind: inferVariableKind(value, key),
  });
  await (variableScope.value === 'grid'
    ? loadGridVariables()
    : loadFileVariables());
  if (variableScope.value === 'file') {
    resetVariableForm();
  }
  ElMessage.success(`变量 ${sanitizeVariableKey(key)} 已保存`);
}

async function deleteVariable(row: AssetVariableRecord) {
  await ElMessageBox.confirm(`确认删除变量 ${row.key}？`, '删除确认');
  await assetVariableDeleteApi(row.id);
  await (variableScope.value === 'grid'
    ? loadGridVariables()
    : loadFileVariables());
}

function search() {
  page.o = 0;
  loadItems();
}

function visibleValues(values: string[], limit = 3) {
  return values.slice(0, limit);
}

function hiddenCount(values: string[], limit = 3) {
  return Math.max(values.length - limit, 0);
}

function formatSize(size: number) {
  if (!size) {
    return '-';
  }
  if (size < 1024) {
    return `${size} B`;
  }
  if (size < 1024 * 1024) {
    return `${(size / 1024).toFixed(1)} KB`;
  }
  return `${(size / 1024 / 1024).toFixed(1)} MB`;
}

function validationTagType(status: string) {
  if (status === 'error') {
    return 'danger';
  }
  if (status === 'warning') {
    return 'warning';
  }
  if (status === 'valid') {
    return 'success';
  }
  return 'info';
}

function validationText(status: string) {
  if (status === 'error') {
    return '错误';
  }
  if (status === 'warning') {
    return '警告';
  }
  if (status === 'valid') {
    return '正常';
  }
  return '待校验';
}

function sourceTagText(source: string) {
  if (source === 'manual') {
    return '手动';
  }
  if (source === 'selection') {
    return '选中';
  }
  return 'AI';
}

function sourceTagType(source: string) {
  if (source === 'manual') {
    return 'success';
  }
  if (source === 'selection') {
    return 'primary';
  }
  return 'warning';
}

function guessVariableKey(value: string) {
  if (/^https?:\/\//.test(value)) {
    return 'SERVICE_URL';
  }
  if (/^\d+:\d+$/.test(value)) {
    return `PORT_${value.split(':')[0]}`;
  }
  if (value.includes(':') && !value.includes('/')) {
    return `${sanitizeVariableKey(value.split(':')[0] || 'IMAGE')}_IMAGE`;
  }
  if (value.includes('/')) {
    const part = value.split('/').findLast(Boolean) || 'PATH';
    return `${sanitizeVariableKey(part)}_PATH`;
  }
  return sanitizeVariableKey(value.slice(0, 32));
}

function sanitizeVariableKey(value: string) {
  const key = value
    .trim()
    .replaceAll(/[^a-z0-9]+/gi, '_')
    .replaceAll(/^_+|_+$/g, '')
    .toUpperCase();
  if (!key) {
    return 'VALUE';
  }
  return /^\d/.test(key) ? `VALUE_${key}` : key;
}

function inferVariableKind(value: string, key = '') {
  const trimmed = value.trim();
  if (/^https?:\/\//.test(trimmed)) {
    return 'url';
  }
  if (/^\d+:\d+$/.test(trimmed)) {
    return 'port';
  }
  if (trimmed.includes(':') && !trimmed.includes('/')) {
    return 'image';
  }
  if (
    trimmed.startsWith('/') ||
    trimmed.startsWith('./') ||
    trimmed.startsWith('~/')
  ) {
    return 'path';
  }
  if (/password|secret|token|key/i.test(key)) {
    return 'secret';
  }
  return 'text';
}

function mergeCandidates(candidates: AssetVariableCandidate[]) {
  const map = new Map<string, AssetVariableCandidate>();
  for (const candidate of candidates) {
    const id = `${candidate.key}\n${candidate.value}`;
    const current = map.get(id);
    if (current) {
      current.occurrences += candidate.occurrences || 1;
    } else {
      map.set(id, { ...candidate, occurrences: candidate.occurrences || 1 });
    }
  }
  return [...map.values()].sort((left, right) =>
    left.key.localeCompare(right.key),
  );
}

onMounted(loadItems);
</script>

<template>
  <Page
    description="按源文件同步本机 YAML 和 Docker Compose 栈"
    title="Docker Compose 管理"
  >
    <div class="space-y-3">
      <div class="flex flex-wrap items-center gap-2 rounded border p-2">
        <ElInput
          v-model="rootPath"
          placeholder="部署目录，默认 ~/DockerCompose"
          size="small"
          style="width: 260px"
        />
        <ElInput
          v-model="keyword"
          clearable
          placeholder="名称 / 服务 / 镜像 / 端口 / 变量"
          size="small"
          style="width: 280px"
          @keyup.enter="search"
        />
        <ElSelect
          v-model="category"
          clearable
          collapse-tags
          collapse-tags-tooltip
          filterable
          multiple
          placeholder="tag"
          size="small"
          style="width: 220px"
        >
          <ElOption
            v-for="item in categories"
            :key="item"
            :label="item"
            :value="item"
          />
        </ElSelect>
        <ElSelect
          v-model="status"
          clearable
          placeholder="常用状态"
          size="small"
          style="width: 128px"
        >
          <ElOption
            v-for="item in assetUsageOptions"
            :key="item.value"
            :label="item.label"
            :value="item.value"
          />
        </ElSelect>
        <ElButton size="small" @click="search">查询</ElButton>
        <ElButton
          :loading="syncing"
          size="small"
          type="primary"
          @click="importDirectory"
        >
          扫描导入
        </ElButton>
        <ElButton size="small" @click="openVariableManager('grid')">
          页面级全局变量
        </ElButton>
        <ElButton
          :loading="refreshingGlobals"
          size="small"
          @click="refreshPageGlobalVariables"
        >
          刷新公共变量
        </ElButton>
        <ElButton size="small" @click="openCreate">新增 Compose</ElButton>
      </div>

      <div
        class="grid grid-cols-[repeat(auto-fill,minmax(320px,1fr))] gap-3"
        v-loading="loading"
      >
        <div
          v-for="item in items"
          :key="item.id"
          class="min-h-[210px] rounded border bg-[var(--el-bg-color)] p-3 text-left transition hover:border-[var(--el-color-primary)] hover:shadow-sm"
          role="button"
          tabindex="0"
          @click="openEdit(item)"
          @keydown.enter="openEdit(item)"
        >
          <div class="mb-2 flex items-start justify-between gap-2">
            <div class="min-w-0">
              <div class="truncate text-sm font-semibold">{{ item.name }}</div>
            </div>
            <ElTag
              :type="validationTagType(item.validationStatus)"
              size="small"
            >
              {{ validationText(item.validationStatus) }}
            </ElTag>
          </div>

          <div class="mb-2 flex flex-wrap gap-1">
            <ElTag
              :type="item.serviceCount > 0 ? 'success' : 'info'"
              size="small"
            >
              {{ item.serviceCount > 0 ? 'Compose' : 'YAML' }}
            </ElTag>
            <ElTag
              :type="item.status === 'enabled' ? 'success' : 'info'"
              size="small"
            >
              {{ assetUsageText(item.status) }}
            </ElTag>
            <ElTag
              v-for="tag in displayTags(item.category, item.tags)"
              :key="tag"
              size="small"
              type="info"
            >
              {{ tag }}
            </ElTag>
            <ElTag size="small" type="info">
              {{ formatSize(item.sourceSize) }}
            </ElTag>
            <ElTag
              v-if="item.variableCandidates.length > 0"
              size="small"
              type="warning"
            >
              变量 {{ item.variableCandidates.length }}
            </ElTag>
          </div>

          <div class="mb-2 min-h-7">
            <ElTag
              v-for="service in visibleValues(item.services, 4)"
              :key="service"
              class="mb-1 mr-1"
              size="small"
            >
              {{ service }}
            </ElTag>
            <ElTag
              v-if="hiddenCount(item.services, 4)"
              size="small"
              type="info"
            >
              +{{ hiddenCount(item.services, 4) }}
            </ElTag>
          </div>

          <div class="space-y-1 text-xs text-[var(--el-text-color-secondary)]">
            <div class="truncate">
              镜像：{{ item.images.slice(0, 2).join(', ') || '-' }}
            </div>
            <div class="truncate">
              端口：{{ item.ports.slice(0, 4).join(', ') || '-' }}
            </div>
            <div class="truncate">
              同步：{{
                item.lastSyncedAt ? formatTime(item.lastSyncedAt) : '-'
              }}
            </div>
          </div>

          <div class="mt-3 flex justify-end gap-2 border-t pt-2">
            <ElButton
              :loading="deploying"
              link
              size="small"
              type="primary"
              @click.stop="deployItem(item)"
            >
              部署
            </ElButton>
            <ElButton
              link
              size="small"
              type="warning"
              @click.stop="toggleItem(item)"
            >
              {{ nextAssetUsageText(item.status) }}
            </ElButton>
            <ElButton
              link
              size="small"
              type="danger"
              @click.stop="deleteItem(item)"
            >
              删除
            </ElButton>
          </div>
        </div>
      </div>

      <div v-if="!loading && items.length === 0" class="rounded border py-8">
        <ElEmpty description="暂无 YAML" />
      </div>

      <div class="flex justify-end">
        <ElPagination
          :page-size="page.s"
          :total="page.total"
          layout="total, sizes, prev, pager, next"
          @current-change="
            (current) => {
              page.o = (current - 1) * page.s;
              loadItems();
            }
          "
          @size-change="
            (size) => {
              page.s = size;
              page.o = 0;
              loadItems();
            }
          "
        />
      </div>
    </div>

    <ElDrawer
      v-model="editorVisible"
      :title="dialogTitle"
      destroy-on-close
      size="88%"
    >
      <div class="grid h-full grid-cols-[minmax(0,1fr)_340px] gap-3">
        <div class="min-w-0 space-y-2">
          <ElForm
            ref="composeFormRef"
            :model="form"
            :rules="composeFormRules"
            class="flex flex-wrap items-start gap-2"
            label-width="0"
          >
            <ElFormItem
              class="mb-0 w-80"
              label="名称"
              label-width="58px"
              prop="name"
            >
              <ElInput v-model="form.name" class="w-full" placeholder="名称" />
            </ElFormItem>
            <ElFormItem class="mb-0 w-56">
              <ElSelect
                v-model="form.tags"
                allow-create
                class="w-full"
                default-first-option
                filterable
                multiple
                placeholder="tag（可选）"
              />
            </ElFormItem>
            <ElFormItem class="mb-0 w-32">
              <ElSelect
                v-model="form.status"
                class="w-full"
                placeholder="常用状态"
              >
                <ElOption
                  v-for="item in assetUsageOptions"
                  :key="item.value"
                  :label="item.label"
                  :value="item.value"
                />
              </ElSelect>
            </ElFormItem>
            <ElFormItem class="mb-0 w-36">
              <ElSelect v-model="extractionScope" class="w-full">
                <ElOption label="文件变量" value="file" />
                <ElOption label="页面级全局变量" value="grid" />
              </ElSelect>
            </ElFormItem>
            <ElButton class="self-start" @click="extractSelectionAsVariable">
              提取选中变量
            </ElButton>
            <ElButton class="self-start" @click="openVariableManager('file')">
              文件变量
            </ElButton>
            <ElButton class="self-start" type="primary" @click="saveItem()">
              保存到库
            </ElButton>
            <ElButton
              :loading="deploying"
              class="self-start"
              type="success"
              @click="deployEditingItem"
            >
              部署
            </ElButton>
          </ElForm>
          <MonacoYamlEditor
            ref="editorRef"
            v-model="form.content"
            height="calc(100vh - 190px)"
            @selection-change="selectedText = $event"
          />
        </div>

        <aside class="min-w-0 space-y-3 overflow-auto pr-1">
          <section class="rounded border p-3">
            <div class="mb-2 flex items-center justify-between">
              <div class="text-sm font-medium">校验</div>
              <ElTag
                :type="validationTagType(form.validationStatus)"
                size="small"
              >
                {{ validationText(form.validationStatus) }}
              </ElTag>
            </div>
            <div v-if="form.validationIssues.length > 0" class="space-y-2">
              <div
                v-for="issue in form.validationIssues"
                :key="`${issue.path}-${issue.message}`"
                class="rounded bg-[var(--el-fill-color-light)] p-2 text-xs"
              >
                <ElTag
                  :type="issue.severity === 'error' ? 'danger' : 'warning'"
                  size="small"
                >
                  {{ issue.severity === 'error' ? '错误' : '警告' }}
                </ElTag>
                <div class="mt-1">{{ issue.message }}</div>
                <div class="mt-1 text-[var(--el-text-color-secondary)]">
                  {{ issue.path }}
                </div>
              </div>
            </div>
            <div v-else class="text-xs text-[var(--el-text-color-secondary)]">
              保存后会刷新 YAML / Compose 校验结果
            </div>
          </section>

          <section class="rounded border p-3">
            <div class="mb-2 flex items-center justify-between">
              <div class="text-sm font-medium">变量候选</div>
              <ElTag size="small" type="info">
                {{ form.variableCandidates.length }}
              </ElTag>
            </div>
            <div v-if="form.variableCandidates.length > 0" class="space-y-2">
              <div
                v-for="candidate in form.variableCandidates"
                :key="`${candidate.key}-${candidate.value}`"
                class="rounded bg-[var(--el-fill-color-light)] p-2"
              >
                <div class="flex items-center justify-between gap-2">
                  <code class="truncate text-xs font-semibold">{{
                    candidate.key
                  }}</code>
                  <ElTag size="small" type="info">{{ candidate.kind }}</ElTag>
                </div>
                <div
                  class="mt-1 line-clamp-2 text-xs text-[var(--el-text-color-secondary)]"
                >
                  {{ candidate.value }}
                </div>
                <div class="mt-2 flex items-center justify-between gap-2">
                  <span class="text-xs text-[var(--el-text-color-secondary)]">
                    {{ candidate.occurrences || 1 }} 次
                  </span>
                  <div class="flex items-center gap-2">
                    <ElButton
                      link
                      size="small"
                      type="primary"
                      @click="promoteCandidate(candidate)"
                    >
                      提升为全局变量
                    </ElButton>
                  </div>
                </div>
              </div>
            </div>
            <div v-else class="text-xs text-[var(--el-text-color-secondary)]">
              可选中字符串后点击“提取选中变量”
            </div>
          </section>

          <section class="rounded border p-3">
            <div class="mb-2 flex items-center justify-between">
              <div class="text-sm font-medium">变量库</div>
              <ElButton link size="small" @click="openVariableManager('grid')">
                页面级管理
              </ElButton>
            </div>
            <div
              class="space-y-1 text-xs text-[var(--el-text-color-secondary)]"
            >
              <div>页面级全局：{{ gridVariables.length }}</div>
              <div>当前文件：{{ fileVariables.length }}</div>
            </div>
          </section>

          <section class="rounded border p-3 text-xs">
            <div class="mb-2 text-sm font-medium">文件</div>
            <div class="space-y-1 text-[var(--el-text-color-secondary)]">
              <div>名称：{{ form.name || form.fileName || '-' }}</div>
              <div>默认部署目录：{{ defaultDeployRoot }}</div>
              <div>大小：{{ formatSize(form.sourceSize) }}</div>
              <div>服务：{{ form.serviceCount }}</div>
              <div>镜像：{{ form.images.length }}</div>
              <div>端口：{{ form.ports.length }}</div>
              <div>
                同步：{{
                  form.lastSyncedAt ? formatTime(form.lastSyncedAt) : '-'
                }}
              </div>
            </div>
          </section>
        </aside>
      </div>
    </ElDrawer>

    <ElDrawer
      v-model="variableVisible"
      :title="variableTitle"
      destroy-on-close
      size="min(620px, calc(100vw - 24px))"
      @closed="handleVariableDrawerClosed"
    >
      <div class="space-y-3">
        <section class="space-y-2 rounded border p-3">
          <ElInput v-model="variableForm.key" placeholder="变量名（必填）" />
          <ElInput v-model="variableForm.value" placeholder="当前值（必填）" />
          <div class="flex justify-end gap-2">
            <ElButton @click="resetVariableForm">清空</ElButton>
            <ElButton type="primary" @click="saveVariable">保存变量</ElButton>
          </div>
        </section>

        <section class="space-y-2">
          <div
            v-for="row in variableRows"
            :key="row.id"
            class="rounded border p-3 text-sm"
          >
            <div class="flex items-start justify-between gap-2">
              <div class="min-w-0">
                <div class="flex items-center gap-2">
                  <code class="font-semibold">{{ row.key }}</code>
                  <ElTag size="small" type="info">{{ row.valueKind }}</ElTag>
                  <ElTag :type="sourceTagType(row.source)" size="small">
                    {{ sourceTagText(row.source) }}
                  </ElTag>
                </div>
                <div
                  class="mt-1 truncate text-xs text-[var(--el-text-color-secondary)]"
                >
                  {{ row.value || row.defaultValue || '-' }}
                </div>
              </div>
              <div class="shrink-0">
                <ElButton
                  v-if="variableScope === 'file'"
                  link
                  size="small"
                  type="primary"
                  @click="promoteVariable(row)"
                >
                  提升为全局变量
                </ElButton>
                <ElButton
                  link
                  size="small"
                  type="danger"
                  @click="deleteVariable(row)"
                >
                  删除
                </ElButton>
              </div>
            </div>
          </div>
          <ElEmpty v-if="variableRows.length === 0" description="暂无变量" />
        </section>
      </div>
    </ElDrawer>

    <ElDrawer
      v-model="deployVisible"
      destroy-on-close
      size="92%"
      title="部署冲突"
    >
      <div v-if="deployPreview" class="flex h-full flex-col gap-3">
        <div
          class="flex flex-wrap items-center justify-between gap-2 rounded border bg-[var(--el-fill-color-lighter)] p-2 text-sm"
        >
          <div class="min-w-0">
            <span class="text-[var(--el-text-color-secondary)]"> 目标： </span>
            <code class="break-all">{{
              deployPreview.targetRelativePath
            }}</code>
          </div>
          <div class="flex shrink-0 items-center gap-2">
            <ElButton size="small" @click="useLibraryDeployContent">
              使用库内容
            </ElButton>
            <ElButton size="small" @click="useLocalDeployContent">
              使用本地内容
            </ElButton>
            <ElButton
              :loading="deploying"
              size="small"
              type="primary"
              @click="saveDeployMerge"
            >
              写入部署
            </ElButton>
          </div>
        </div>

        <div class="deploy-merge-grid flex min-h-0 flex-1 flex-col gap-3">
          <MonacoYamlDiffEditor
            :modified-value="deployPreview.localContent"
            :original-value="deployPreview.libraryContent"
            height="min(52vh, 560px)"
          />
          <section class="flex min-h-0 flex-1 flex-col gap-2">
            <div class="text-sm font-medium">合并结果</div>
            <ElInput
              v-model="deployMergedContent"
              class="min-h-0 flex-1"
              resize="none"
              type="textarea"
            />
          </section>
        </div>
      </div>
    </ElDrawer>
  </Page>
</template>

<style scoped>
.deploy-merge-grid :deep(.el-textarea),
.deploy-merge-grid :deep(.el-textarea__inner) {
  height: 100%;
}

.deploy-merge-grid :deep(.el-textarea__inner) {
  min-height: 100%;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas,
    'Liberation Mono', 'Courier New', monospace;
  font-size: 12px;
  line-height: 1.45;
}
</style>
