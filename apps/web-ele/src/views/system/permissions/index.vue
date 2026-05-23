<script lang="ts" setup>
import { onMounted, reactive, ref } from 'vue';

import { Page } from '@vben/common-ui';

import {
  ElButton,
  ElDialog,
  ElForm,
  ElFormItem,
  ElInput,
  ElInputNumber,
  ElOption,
  ElSelect,
  ElTable,
  ElTableColumn,
  ElTag,
} from 'element-plus';

import {
  type PermissionRecord,
  permissionSaveApi,
  permissionTreeApi,
} from '#/api';

import { statusOptions } from '../shared';

const permissions = ref<PermissionRecord[]>([]);
const editorVisible = ref(false);
const form = reactive({
  code: '',
  component: '',
  icon: '',
  id: '',
  name: '',
  parentId: '',
  path: '',
  permissionType: 'menu',
  sortOrder: 0,
  status: 'enabled',
});

async function loadPermissions() {
  permissions.value = await permissionTreeApi();
}

function openCreate() {
  Object.assign(form, {
    code: '',
    component: '',
    icon: '',
    id: '',
    name: '',
    parentId: '',
    path: '',
    permissionType: 'menu',
    sortOrder: 0,
    status: 'enabled',
  });
  editorVisible.value = true;
}

function openEdit(row: PermissionRecord) {
  Object.assign(form, {
    ...row,
    parentId: row.parentId ?? '',
  });
  editorVisible.value = true;
}

async function savePermission() {
  await permissionSaveApi({
    ...form,
    id: form.id || undefined,
    parentId: form.parentId || undefined,
  });
  editorVisible.value = false;
  await loadPermissions();
}

onMounted(loadPermissions);
</script>

<template>
  <Page description="维护动态菜单节点和按钮权限码" title="权限管理">
    <div class="space-y-3">
      <div class="flex justify-end">
        <ElButton type="primary" @click="openCreate">新增权限</ElButton>
      </div>

      <ElTable :data="permissions" border size="small" stripe>
        <ElTableColumn label="名称" min-width="140" prop="name" />
        <ElTableColumn label="编码" min-width="180" prop="code" />
        <ElTableColumn label="类型" width="90">
          <template #default="{ row }">
            <ElTag>{{ row.permissionType === 'menu' ? '菜单' : '按钮' }}</ElTag>
          </template>
        </ElTableColumn>
        <ElTableColumn label="路径" min-width="180" prop="path" />
        <ElTableColumn label="组件" min-width="220" prop="component" />
        <ElTableColumn label="排序" prop="sortOrder" width="80" />
        <ElTableColumn fixed="right" label="操作" width="110">
          <template #default="{ row }">
            <ElButton link type="primary" @click="openEdit(row)">编辑</ElButton>
          </template>
        </ElTableColumn>
      </ElTable>
    </div>

    <ElDialog v-model="editorVisible" title="权限节点" width="760px">
      <ElForm :model="form" class="grid grid-cols-2 gap-x-4" label-width="90px">
        <ElFormItem label="名称">
          <ElInput v-model="form.name" />
        </ElFormItem>
        <ElFormItem label="编码">
          <ElInput v-model="form.code" />
        </ElFormItem>
        <ElFormItem label="类型">
          <ElSelect v-model="form.permissionType" class="w-full">
            <ElOption label="菜单" value="menu" />
            <ElOption label="按钮" value="button" />
          </ElSelect>
        </ElFormItem>
        <ElFormItem label="父级">
          <ElSelect v-model="form.parentId" class="w-full" clearable filterable>
            <ElOption
              v-for="permission in permissions"
              :key="permission.id"
              :label="permission.name"
              :value="permission.id"
            />
          </ElSelect>
        </ElFormItem>
        <ElFormItem label="路径">
          <ElInput v-model="form.path" />
        </ElFormItem>
        <ElFormItem label="组件">
          <ElInput v-model="form.component" />
        </ElFormItem>
        <ElFormItem label="图标">
          <ElInput v-model="form.icon" />
        </ElFormItem>
        <ElFormItem label="排序">
          <ElInputNumber v-model="form.sortOrder" class="w-full" />
        </ElFormItem>
        <ElFormItem label="状态">
          <ElSelect v-model="form.status" class="w-full">
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
        <ElButton @click="editorVisible = false">取消</ElButton>
        <ElButton type="primary" @click="savePermission">保存</ElButton>
      </template>
    </ElDialog>
  </Page>
</template>
