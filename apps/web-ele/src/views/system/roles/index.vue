<script lang="ts" setup>
import { computed, onMounted, reactive, ref } from 'vue';

import { Page } from '@vben/common-ui';

import {
  ElButton,
  ElDialog,
  ElForm,
  ElFormItem,
  ElInput,
  ElMessageBox,
  ElOption,
  ElPagination,
  ElSelect,
  ElTable,
  ElTableColumn,
  ElTag,
} from 'element-plus';

import {
  type PermissionRecord,
  permissionTreeApi,
  roleAssignPermissionsApi,
  roleCreateApi,
  roleDeleteApi,
  rolePageApi,
  rolePermissionIdsApi,
  type RoleRecord,
  roleUpdateApi,
} from '#/api';

import { formatTime, statusOptions } from '../shared';

const loading = ref(false);
const roles = ref<RoleRecord[]>([]);
const permissions = ref<PermissionRecord[]>([]);
const keyword = ref('');
const page = reactive({ o: 0, s: 10, total: 0 });
const editorVisible = ref(false);
const assignVisible = ref(false);
const assignLoading = ref(false);
const editingId = ref('');
const assigningRoleId = ref('');
const assigningRoleName = ref('');
const selectedButtonPermissionIds = ref<string[]>([]);
const selectedMenuPermissionIds = ref<string[]>([]);
const form = reactive({
  code: '',
  description: '',
  name: '',
  status: 'enabled',
});
const title = computed(() => (editingId.value ? '编辑角色' : '新增角色'));
const assignTitle = computed(() =>
  assigningRoleName.value
    ? `菜单授权 - ${assigningRoleName.value}`
    : '菜单授权',
);
const menuPermissions = computed(() =>
  permissions.value.filter(
    (permission) => permission.permissionType === 'menu',
  ),
);
const buttonPermissions = computed(() =>
  permissions.value.filter(
    (permission) => permission.permissionType === 'button',
  ),
);

async function loadRoles() {
  loading.value = true;
  try {
    const result = await rolePageApi({
      keyword: keyword.value,
      o: page.o,
      s: page.s,
    });
    roles.value = result.d;
    page.total = result.t;
  } finally {
    loading.value = false;
  }
}

async function loadPermissions() {
  permissions.value = await permissionTreeApi();
}

function openCreate() {
  editingId.value = '';
  Object.assign(form, {
    code: '',
    description: '',
    name: '',
    status: 'enabled',
  });
  editorVisible.value = true;
}

function openEdit(row: RoleRecord) {
  editingId.value = row.id;
  Object.assign(form, row);
  editorVisible.value = true;
}

async function saveRole() {
  await (editingId.value
    ? roleUpdateApi({ id: editingId.value, ...form })
    : roleCreateApi(form));
  editorVisible.value = false;
  await loadRoles();
}

async function openAssign(row: RoleRecord) {
  assigningRoleId.value = row.id;
  assigningRoleName.value = row.name;
  selectedButtonPermissionIds.value = [];
  selectedMenuPermissionIds.value = [];
  assignVisible.value = true;

  assignLoading.value = true;
  try {
    if (permissions.value.length === 0) {
      await loadPermissions();
    }
    const selectedIds = await rolePermissionIdsApi(row.id);
    const selectedIdSet = new Set(selectedIds);
    selectedMenuPermissionIds.value = menuPermissions.value
      .filter((permission) => selectedIdSet.has(permission.id))
      .map((permission) => permission.id);
    selectedButtonPermissionIds.value = buttonPermissions.value
      .filter((permission) => selectedIdSet.has(permission.id))
      .map((permission) => permission.id);
  } finally {
    assignLoading.value = false;
  }
}

async function saveAssign() {
  await roleAssignPermissionsApi({
    permissionIds: [
      ...selectedMenuPermissionIds.value,
      ...selectedButtonPermissionIds.value,
    ],
    roleId: assigningRoleId.value,
  });
  assignVisible.value = false;
}

async function deleteRole(row: RoleRecord) {
  await ElMessageBox.confirm(`确认删除角色 ${row.name}？`, '删除确认');
  await roleDeleteApi(row.id);
  await loadRoles();
}

function search() {
  page.o = 0;
  loadRoles();
}

onMounted(async () => {
  await Promise.all([loadRoles(), loadPermissions()]);
});
</script>

<template>
  <Page description="维护角色并分配菜单和按钮权限" title="角色管理">
    <div class="space-y-3">
      <div class="flex items-center justify-between gap-3">
        <div class="flex items-center gap-2">
          <ElInput
            v-model="keyword"
            class="w-64"
            clearable
            placeholder="角色编码 / 名称"
            @keyup.enter="search"
          />
          <ElButton @click="search">查询</ElButton>
        </div>
        <ElButton type="primary" @click="openCreate">新增角色</ElButton>
      </div>

      <ElTable :data="roles" border size="small" stripe v-loading="loading">
        <ElTableColumn label="编码" min-width="140" prop="code" />
        <ElTableColumn label="名称" min-width="140" prop="name" />
        <ElTableColumn label="描述" min-width="220" prop="description" />
        <ElTableColumn label="状态" width="90">
          <template #default="{ row }">
            <ElTag :type="row.status === 'enabled' ? 'success' : 'info'">
              {{ row.status === 'enabled' ? '启用' : '禁用' }}
            </ElTag>
          </template>
        </ElTableColumn>
        <ElTableColumn label="更新时间" width="180">
          <template #default="{ row }">
            {{ formatTime(row.updatedAt) }}
          </template>
        </ElTableColumn>
        <ElTableColumn fixed="right" label="操作" width="220">
          <template #default="{ row }">
            <ElButton link type="primary" @click="openEdit(row)">编辑</ElButton>
            <ElButton link type="success" @click="openAssign(row)">
              菜单授权
            </ElButton>
            <ElButton link type="danger" @click="deleteRole(row)">
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
              loadRoles();
            }
          "
          @size-change="
            (size) => {
              page.s = size;
              page.o = 0;
              loadRoles();
            }
          "
        />
      </div>
    </div>

    <ElDialog v-model="editorVisible" :title="title" width="640px">
      <ElForm :model="form" class="grid grid-cols-2 gap-x-4" label-width="90px">
        <ElFormItem label="编码">
          <ElInput v-model="form.code" />
        </ElFormItem>
        <ElFormItem label="名称">
          <ElInput v-model="form.name" />
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
        <ElFormItem class="col-span-2" label="描述">
          <ElInput v-model="form.description" type="textarea" />
        </ElFormItem>
      </ElForm>
      <template #footer>
        <ElButton @click="editorVisible = false">取消</ElButton>
        <ElButton type="primary" @click="saveRole">保存</ElButton>
      </template>
    </ElDialog>

    <ElDialog v-model="assignVisible" :title="assignTitle" width="760px">
      <div class="space-y-4" v-loading="assignLoading">
        <section class="rounded border border-[var(--el-border-color)] p-3">
          <div class="mb-2 flex items-center justify-between">
            <div class="text-sm font-medium">菜单权限</div>
            <ElTag size="small" type="info">
              {{ selectedMenuPermissionIds.length }} /
              {{ menuPermissions.length }}
            </ElTag>
          </div>
          <ElSelect
            v-model="selectedMenuPermissionIds"
            class="w-full"
            collapse-tags
            collapse-tags-tooltip
            filterable
            multiple
            placeholder="选择可见菜单"
          >
            <ElOption
              v-for="permission in menuPermissions"
              :key="permission.id"
              :label="`${permission.name} (${permission.code})`"
              :value="permission.id"
            />
          </ElSelect>
        </section>

        <section class="rounded border border-[var(--el-border-color)] p-3">
          <div class="mb-2 flex items-center justify-between">
            <div class="text-sm font-medium">按钮权限码</div>
            <ElTag size="small" type="info">
              {{ selectedButtonPermissionIds.length }} /
              {{ buttonPermissions.length }}
            </ElTag>
          </div>
          <ElSelect
            v-model="selectedButtonPermissionIds"
            class="w-full"
            collapse-tags
            collapse-tags-tooltip
            filterable
            multiple
            placeholder="选择按钮权限码"
          >
            <ElOption
              v-for="permission in buttonPermissions"
              :key="permission.id"
              :label="`${permission.name} (${permission.code})`"
              :value="permission.id"
            />
          </ElSelect>
        </section>
      </div>
      <template #footer>
        <ElButton @click="assignVisible = false">取消</ElButton>
        <ElButton type="primary" @click="saveAssign">保存</ElButton>
      </template>
    </ElDialog>
  </Page>
</template>
