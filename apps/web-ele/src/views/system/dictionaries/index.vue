<script lang="ts" setup>
import { onMounted, reactive, ref, watch } from 'vue';

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
  ElSelect,
  ElTable,
  ElTableColumn,
  ElTag,
} from 'element-plus';

import {
  dictItemCreateApi,
  dictItemDeleteApi,
  dictItemPageApi,
  type DictItemRecord,
  dictItemUpdateApi,
  dictTypeCreateApi,
  dictTypeDeleteApi,
  dictTypePageApi,
  type DictTypeRecord,
  dictTypeUpdateApi,
} from '#/api';

import { statusOptions } from '../shared';

const types = ref<DictTypeRecord[]>([]);
const items = ref<DictItemRecord[]>([]);
const selectedTypeId = ref('');
const typeDialog = ref(false);
const itemDialog = ref(false);
const editingTypeId = ref('');
const editingItemId = ref('');
const typeForm = reactive({
  code: '',
  description: '',
  name: '',
  sortOrder: 0,
  status: 'enabled',
});
const itemForm = reactive({
  label: '',
  sortOrder: 0,
  status: 'enabled',
  typeId: '',
  value: '',
});

async function loadTypes() {
  const result = await dictTypePageApi({ o: 0, s: 100 });
  types.value = result.d;
  if (!selectedTypeId.value && types.value[0]) {
    selectedTypeId.value = types.value[0].id;
  }
}

async function loadItems() {
  if (!selectedTypeId.value) {
    items.value = [];
    return;
  }
  const result = await dictItemPageApi({
    o: 0,
    s: 100,
    typeId: selectedTypeId.value,
  });
  items.value = result.d;
}

function openTypeCreate() {
  editingTypeId.value = '';
  Object.assign(typeForm, {
    code: '',
    description: '',
    name: '',
    sortOrder: 0,
    status: 'enabled',
  });
  typeDialog.value = true;
}

function openTypeEdit(row: DictTypeRecord) {
  editingTypeId.value = row.id;
  Object.assign(typeForm, row);
  typeDialog.value = true;
}

async function saveType() {
  await (editingTypeId.value
    ? dictTypeUpdateApi({ id: editingTypeId.value, ...typeForm })
    : dictTypeCreateApi(typeForm));
  typeDialog.value = false;
  await loadTypes();
}

async function deleteType(row: DictTypeRecord) {
  await ElMessageBox.confirm(`确认删除字典 ${row.name}？`, '删除确认');
  await dictTypeDeleteApi(row.id);
  if (selectedTypeId.value === row.id) {
    selectedTypeId.value = '';
  }
  await loadTypes();
  await loadItems();
}

function openItemCreate() {
  editingItemId.value = '';
  Object.assign(itemForm, {
    label: '',
    sortOrder: 0,
    status: 'enabled',
    typeId: selectedTypeId.value,
    value: '',
  });
  itemDialog.value = true;
}

function openItemEdit(row: DictItemRecord) {
  editingItemId.value = row.id;
  Object.assign(itemForm, row);
  itemDialog.value = true;
}

async function saveItem() {
  await (editingItemId.value
    ? dictItemUpdateApi({ id: editingItemId.value, ...itemForm })
    : dictItemCreateApi(itemForm));
  itemDialog.value = false;
  await loadItems();
}

async function deleteItem(row: DictItemRecord) {
  await ElMessageBox.confirm(`确认删除字典项 ${row.label}？`, '删除确认');
  await dictItemDeleteApi(row.id);
  await loadItems();
}

watch(selectedTypeId, loadItems);

onMounted(async () => {
  await loadTypes();
  await loadItems();
});
</script>

<template>
  <Page description="维护本地基础编码、选项和值集" title="字典管理">
    <div class="grid grid-cols-[460px_minmax(0,1fr)] gap-4">
      <section class="border-border space-y-3 rounded border p-3">
        <div class="flex items-center justify-between">
          <span class="font-medium">字典类型</span>
          <ElButton size="small" type="primary" @click="openTypeCreate">
            新增
          </ElButton>
        </div>
        <ElTable
          :data="types"
          border
          highlight-current-row
          size="small"
          @row-click="(row) => (selectedTypeId = row.id)"
        >
          <ElTableColumn label="名称" min-width="120" prop="name" />
          <ElTableColumn label="编码" min-width="140" prop="code" />
          <ElTableColumn fixed="right" label="操作" width="120">
            <template #default="{ row }">
              <ElButton link type="primary" @click.stop="openTypeEdit(row)">
                编辑
              </ElButton>
              <ElButton link type="danger" @click.stop="deleteType(row)">
                删除
              </ElButton>
            </template>
          </ElTableColumn>
        </ElTable>
      </section>

      <section class="border-border space-y-3 rounded border p-3">
        <div class="flex items-center justify-between">
          <span class="font-medium">字典项</span>
          <ElButton
            :disabled="!selectedTypeId"
            size="small"
            type="primary"
            @click="openItemCreate"
          >
            新增
          </ElButton>
        </div>
        <ElTable :data="items" border size="small" stripe>
          <ElTableColumn label="标签" min-width="160" prop="label" />
          <ElTableColumn label="值" min-width="160" prop="value" />
          <ElTableColumn label="排序" prop="sortOrder" width="90" />
          <ElTableColumn label="状态" width="90">
            <template #default="{ row }">
              <ElTag :type="row.status === 'enabled' ? 'success' : 'info'">
                {{ row.status === 'enabled' ? '启用' : '禁用' }}
              </ElTag>
            </template>
          </ElTableColumn>
          <ElTableColumn fixed="right" label="操作" width="140">
            <template #default="{ row }">
              <ElButton link type="primary" @click="openItemEdit(row)">
                编辑
              </ElButton>
              <ElButton link type="danger" @click="deleteItem(row)">
                删除
              </ElButton>
            </template>
          </ElTableColumn>
        </ElTable>
      </section>
    </div>

    <ElDialog v-model="typeDialog" title="字典类型" width="640px">
      <ElForm
        :model="typeForm"
        class="grid grid-cols-2 gap-x-4"
        label-width="90px"
      >
        <ElFormItem label="编码">
          <ElInput v-model="typeForm.code" />
        </ElFormItem>
        <ElFormItem label="名称">
          <ElInput v-model="typeForm.name" />
        </ElFormItem>
        <ElFormItem label="排序">
          <ElInputNumber v-model="typeForm.sortOrder" class="w-full" />
        </ElFormItem>
        <ElFormItem label="状态">
          <ElSelect v-model="typeForm.status" class="w-full">
            <ElOption
              v-for="item in statusOptions"
              :key="item.value"
              :label="item.label"
              :value="item.value"
            />
          </ElSelect>
        </ElFormItem>
        <ElFormItem class="col-span-2" label="描述">
          <ElInput v-model="typeForm.description" type="textarea" />
        </ElFormItem>
      </ElForm>
      <template #footer>
        <ElButton @click="typeDialog = false">取消</ElButton>
        <ElButton type="primary" @click="saveType">保存</ElButton>
      </template>
    </ElDialog>

    <ElDialog v-model="itemDialog" title="字典项" width="560px">
      <ElForm
        :model="itemForm"
        class="grid grid-cols-2 gap-x-4"
        label-width="80px"
      >
        <ElFormItem label="标签">
          <ElInput v-model="itemForm.label" />
        </ElFormItem>
        <ElFormItem label="值">
          <ElInput v-model="itemForm.value" />
        </ElFormItem>
        <ElFormItem label="排序">
          <ElInputNumber v-model="itemForm.sortOrder" class="w-full" />
        </ElFormItem>
        <ElFormItem label="状态">
          <ElSelect v-model="itemForm.status" class="w-full">
            <ElOption
              v-for="item in statusOptions"
              :key="item.value"
              :label="item.label"
              :value="item.value"
            />
          </ElSelect>
        </ElFormItem>
      </ElForm>
      <template #footer>
        <ElButton @click="itemDialog = false">取消</ElButton>
        <ElButton type="primary" @click="saveItem">保存</ElButton>
      </template>
    </ElDialog>
  </Page>
</template>
