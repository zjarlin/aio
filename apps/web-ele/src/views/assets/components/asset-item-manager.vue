<script lang="ts" setup>
import type { FormInstance, FormRules } from 'element-plus';

import { computed, onMounted, reactive, ref } from 'vue';

import { Page } from '@vben/common-ui';

import {
  ElButton,
  ElDialog,
  ElForm,
  ElFormItem,
  ElInput,
  ElInputNumber,
  ElMessageBox,
  ElOption,
  ElPagination,
  ElSelect,
  ElTable,
  ElTableColumn,
  ElTag,
} from 'element-plus';

import {
  assetItemCreateApi,
  assetItemDeleteApi,
  type AssetItemKind,
  assetItemPageApi,
  type AssetItemRecord,
  assetItemToggleApi,
  assetItemUpdateApi,
} from '#/api';

import { formatTime } from '../../system/shared';
import {
  assetUsageOptions,
  assetUsageText,
  displayTags,
  nextAssetUsageStatus,
  nextAssetUsageText,
  normalizeTags,
} from '../shared';
import QuickComposer from './quick-composer.vue';

const props = withDefaults(
  defineProps<{
    defaultCategory?: string;
    description: string;
    kind: AssetItemKind;
    quickPlaceholder: string;
    quickSubmitText?: string;
    title: string;
  }>(),
  {
    defaultCategory: '',
    quickSubmitText: '新增',
  },
);

const loading = ref(false);
const items = ref<AssetItemRecord[]>([]);
const keyword = ref('');
const category = ref('');
const status = ref('');
const page = reactive({ o: 0, s: 10, total: 0 });
const editorVisible = ref(false);
const editingId = ref('');
const formRef = ref<FormInstance>();
const form = reactive({
  category: '',
  code: '',
  content: '',
  description: '',
  name: '',
  sortOrder: 0,
  status: 'disabled',
  tags: [] as string[],
});
const formRules: FormRules = {
  name: [{ message: '名称必填', required: true, trigger: 'blur' }],
};

const dialogTitle = computed(() => (editingId.value ? '编辑条目' : '新增条目'));
const categories = computed(() => {
  const values = items.value.flatMap((item) =>
    displayTags(item.category, item.tags),
  );
  return [...new Set(values)];
});

function parseTags(content: string) {
  const matches = content.match(/#[\u4E00-\u9FA5\w-]+/g) ?? [];
  return [...new Set(matches.map((tag) => tag.slice(1)))];
}

function inferTitle(content: string) {
  const firstLine = content
    .split('\n')
    .map((line) => line.trim())
    .find(Boolean);
  return firstLine?.replace(/^#+\s*/, '').slice(0, 48) || '未命名条目';
}

function slugify(value: string) {
  const slug = value
    .trim()
    .toLowerCase()
    .replaceAll(/[^a-z0-9]+/g, '-')
    .replaceAll(/^-+|-+$/g, '');
  return `asset-${props.kind}-${slug || 'item'}-${Date.now().toString(36)}`;
}

async function loadItems() {
  loading.value = true;
  try {
    const result = await assetItemPageApi({
      category: category.value || undefined,
      kind: props.kind,
      keyword: keyword.value,
      o: page.o,
      s: page.s,
      status: status.value || undefined,
    });
    items.value = result.d;
    page.total = result.t;
  } finally {
    loading.value = false;
  }
}

function openCreate() {
  editingId.value = '';
  Object.assign(form, {
    category: '',
    code: '',
    content: '',
    description: '',
    name: '',
    sortOrder: 0,
    status: 'disabled',
    tags: [],
  });
  editorVisible.value = true;
}

async function quickCreate(content: string, done: (success?: boolean) => void) {
  const name = inferTitle(content);
  const tags = parseTags(content);
  try {
    await assetItemCreateApi({
      category: tags[0] || '',
      code: slugify(name),
      content,
      description: content.split('\n').slice(1, 3).join(' ').slice(0, 120),
      kind: props.kind,
      name,
      sortOrder: 0,
      status: 'disabled',
      tags,
    });
    await loadItems();
    done(true);
  } catch {
    done(false);
  }
}

function openEdit(row: AssetItemRecord) {
  editingId.value = row.id;
  Object.assign(form, {
    category: row.category,
    code: row.code,
    content: row.content,
    description: row.description,
    name: row.name,
    sortOrder: row.sortOrder,
    status: row.status,
    tags: displayTags(row.category, row.tags),
  });
  editorVisible.value = true;
}

async function saveItem() {
  await formRef.value?.validate();
  const tags = normalizeTags(form.tags);
  const name = form.name.trim();
  const input = {
    category: tags[0] || '',
    code: form.code.trim() || slugify(name),
    content: form.content,
    description: form.description,
    kind: props.kind,
    name,
    sortOrder: form.sortOrder,
    status: form.status,
    tags,
  };

  await (editingId.value
    ? assetItemUpdateApi({ id: editingId.value, ...input })
    : assetItemCreateApi(input));
  editorVisible.value = false;
  await loadItems();
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

function search() {
  page.o = 0;
  loadItems();
}

onMounted(loadItems);
</script>

<template>
  <Page :description="description" :title="props.title">
    <div class="space-y-4">
      <QuickComposer
        :placeholder="quickPlaceholder"
        :submit-text="quickSubmitText"
        @submit="quickCreate"
      />

      <div class="flex flex-wrap items-center justify-between gap-3">
        <div class="flex flex-wrap items-center gap-2">
          <ElInput
            v-model="keyword"
            class="w-64"
            clearable
            placeholder="名称 / 编码 / 内容"
            @keyup.enter="search"
          />
          <ElSelect
            v-model="category"
            allow-create
            class="w-40"
            clearable
            filterable
            placeholder="tag"
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
            class="w-32"
            clearable
            placeholder="常用状态"
          >
            <ElOption
              v-for="item in assetUsageOptions"
              :key="item.value"
              :label="item.label"
              :value="item.value"
            />
          </ElSelect>
          <ElButton @click="search">查询</ElButton>
        </div>
        <ElButton type="primary" @click="openCreate">新增条目</ElButton>
      </div>

      <ElTable :data="items" border size="small" stripe v-loading="loading">
        <ElTableColumn
          label="名称"
          min-width="150"
          prop="name"
          show-overflow-tooltip
        />
        <ElTableColumn
          label="编码"
          min-width="180"
          prop="code"
          show-overflow-tooltip
        />
        <ElTableColumn label="tag" min-width="180">
          <template #default="{ row }">
            <ElTag
              v-for="tag in displayTags(row.category, row.tags)"
              :key="tag"
              class="mr-1"
              size="small"
              type="info"
            >
              {{ tag }}
            </ElTag>
          </template>
        </ElTableColumn>
        <ElTableColumn
          label="说明"
          min-width="200"
          prop="description"
          show-overflow-tooltip
        />
        <ElTableColumn
          label="内容"
          min-width="240"
          prop="content"
          show-overflow-tooltip
        />
        <ElTableColumn label="排序" prop="sortOrder" width="80" />
        <ElTableColumn label="常用状态" width="100">
          <template #default="{ row }">
            <ElTag :type="row.status === 'enabled' ? 'success' : 'info'">
              {{ assetUsageText(row.status) }}
            </ElTag>
          </template>
        </ElTableColumn>
        <ElTableColumn label="更新时间" width="180">
          <template #default="{ row }">
            {{ formatTime(row.updatedAt) }}
          </template>
        </ElTableColumn>
        <ElTableColumn fixed="right" label="操作" width="210">
          <template #default="{ row }">
            <ElButton link type="primary" @click="openEdit(row)">编辑</ElButton>
            <ElButton link type="warning" @click="toggleItem(row)">
              {{ nextAssetUsageText(row.status) }}
            </ElButton>
            <ElButton link type="danger" @click="deleteItem(row)">
              删除
            </ElButton>
          </template>
        </ElTableColumn>
      </ElTable>

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

    <ElDialog v-model="editorVisible" :title="dialogTitle" width="900px">
      <ElForm
        ref="formRef"
        :model="form"
        :rules="formRules"
        class="grid grid-cols-2 gap-x-4"
        label-width="88px"
      >
        <ElFormItem label="编码">
          <ElInput v-model="form.code" />
        </ElFormItem>
        <ElFormItem label="名称" prop="name">
          <ElInput v-model="form.name" />
        </ElFormItem>
        <ElFormItem label="状态">
          <ElSelect v-model="form.status" class="w-full">
            <ElOption
              v-for="item in assetUsageOptions"
              :key="item.value"
              :label="item.label"
              :value="item.value"
            />
          </ElSelect>
        </ElFormItem>
        <ElFormItem label="tag">
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
        <ElFormItem label="排序">
          <ElInputNumber v-model="form.sortOrder" class="w-full" />
        </ElFormItem>
        <ElFormItem class="col-span-2" label="说明">
          <ElInput v-model="form.description" :rows="2" type="textarea" />
        </ElFormItem>
        <ElFormItem class="col-span-2" label="内容">
          <ElInput v-model="form.content" :rows="12" type="textarea" />
        </ElFormItem>
      </ElForm>
      <template #footer>
        <ElButton @click="editorVisible = false">取消</ElButton>
        <ElButton type="primary" @click="saveItem">保存</ElButton>
      </template>
    </ElDialog>
  </Page>
</template>
