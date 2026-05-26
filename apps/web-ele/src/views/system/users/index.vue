<script lang="ts" setup>
import { computed, onMounted, reactive, ref, watch } from 'vue';

import { Page } from '@vben/common-ui';
import { DEFAULT_HOME_PATH, ORDINARY_USER_HOME_PATH } from '@vben/constants';

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
  rolePageApi,
  type RoleRecord,
  userCreateApi,
  userDeleteApi,
  userDisableApi,
  userPageApi,
  type UserRecord,
  userResetPasswordApi,
  userUpdateApi,
} from '#/api';

import { formatTime, parseRoleCodes, statusOptions } from '../shared';

const loading = ref(false);
const users = ref<UserRecord[]>([]);
const roles = ref<RoleRecord[]>([]);
const keyword = ref('');
const page = reactive({ o: 0, s: 10, total: 0 });
const editorVisible = ref(false);
const passwordVisible = ref(false);
const editingId = ref('');
const form = reactive({
  avatar: '',
  homePath: DEFAULT_HOME_PATH,
  password: '',
  realName: '',
  roleIds: [] as string[],
  status: 'enabled',
  username: '',
});
const passwordForm = reactive({ id: '', password: 'admin123456' });
const title = computed(() => (editingId.value ? '编辑用户' : '新增用户'));

const ORDINARY_USER_ROLE_CODE = 'ordinary_user';
const SUPER_ADMIN_ROLE_CODE = 'super_admin';
const LEGACY_HOME_PATHS = new Set(['/analytics', '/workspace']);

async function loadRoles() {
  const result = await rolePageApi({ o: 0, s: 100 });
  roles.value = result.d;
}

async function loadUsers() {
  loading.value = true;
  try {
    const result = await userPageApi({
      keyword: keyword.value,
      o: page.o,
      s: page.s,
    });
    users.value = result.d;
    page.total = result.t;
  } finally {
    loading.value = false;
  }
}

function openCreate() {
  editingId.value = '';
  Object.assign(form, {
    avatar: '',
    homePath: DEFAULT_HOME_PATH,
    password: 'admin123456',
    realName: '',
    roleIds: [],
    status: 'enabled',
    username: '',
  });
  editorVisible.value = true;
}

function openEdit(row: UserRecord) {
  editingId.value = row.id;
  const roleCodes = parseRoleCodes(row.roles);
  const roleIds = roles.value
    .filter((role) => roleCodes.includes(role.code))
    .map((role) => role.id);
  Object.assign(form, {
    avatar: row.avatar,
    homePath: resolveHomePath(row.homePath, roleCodes),
    password: '',
    realName: row.realName,
    roleIds,
    status: row.status,
    username: row.username,
  });
  editorVisible.value = true;
}

async function saveUser() {
  const payload = {
    ...form,
    homePath: resolveHomePath(form.homePath, selectedRoleCodes(form.roleIds)),
  };
  await (editingId.value
    ? userUpdateApi({ id: editingId.value, ...payload })
    : userCreateApi(payload));
  editorVisible.value = false;
  await loadUsers();
}

function openReset(row: UserRecord) {
  passwordForm.id = row.id;
  passwordForm.password = 'admin123456';
  passwordVisible.value = true;
}

async function resetPassword() {
  await userResetPasswordApi(passwordForm);
  passwordVisible.value = false;
}

async function disableUser(row: UserRecord) {
  await userDisableApi(row.id);
  await loadUsers();
}

async function deleteUser(row: UserRecord) {
  await ElMessageBox.confirm(`确认删除用户 ${row.username}？`, '删除确认');
  await userDeleteApi(row.id);
  await loadUsers();
}

function search() {
  page.o = 0;
  loadUsers();
}

function selectedRoleCodes(roleIds: string[]) {
  const roleIdSet = new Set(roleIds);
  return roles.value
    .filter((role) => roleIdSet.has(role.id))
    .map((role) => role.code);
}

function isOrdinaryUser(roleCodes: string[]) {
  return (
    roleCodes.includes(ORDINARY_USER_ROLE_CODE) &&
    !roleCodes.includes(SUPER_ADMIN_ROLE_CODE)
  );
}

function isDefaultHomePath(homePath: string) {
  return (
    !homePath ||
    LEGACY_HOME_PATHS.has(homePath) ||
    homePath === DEFAULT_HOME_PATH ||
    homePath === ORDINARY_USER_HOME_PATH
  );
}

function resolveHomePath(homePath: string, roleCodes: string[]) {
  if (isOrdinaryUser(roleCodes) && isDefaultHomePath(homePath)) {
    return ORDINARY_USER_HOME_PATH;
  }
  return homePath || DEFAULT_HOME_PATH;
}

watch(
  () => [...form.roleIds],
  (roleIds) => {
    if (editingId.value || !isDefaultHomePath(form.homePath)) {
      return;
    }
    form.homePath = isOrdinaryUser(selectedRoleCodes(roleIds))
      ? ORDINARY_USER_HOME_PATH
      : DEFAULT_HOME_PATH;
  },
);

onMounted(async () => {
  await loadRoles();
  await loadUsers();
});
</script>

<template>
  <Page description="管理本机 AIO 用户、角色绑定和登录状态" title="用户管理">
    <div class="space-y-3">
      <div class="flex items-center justify-between gap-3">
        <div class="flex items-center gap-2">
          <ElInput
            v-model="keyword"
            class="w-64"
            clearable
            placeholder="用户名 / 姓名"
            @keyup.enter="search"
          />
          <ElButton @click="search">查询</ElButton>
        </div>
        <ElButton type="primary" @click="openCreate">新增用户</ElButton>
      </div>

      <ElTable :data="users" border size="small" stripe v-loading="loading">
        <ElTableColumn label="用户名" min-width="120" prop="username" />
        <ElTableColumn label="姓名" min-width="120" prop="realName" />
        <ElTableColumn label="角色" min-width="180">
          <template #default="{ row }">
            <ElTag
              v-for="role in parseRoleCodes(row.roles)"
              :key="role"
              class="mr-1"
              size="small"
            >
              {{ role }}
            </ElTag>
          </template>
        </ElTableColumn>
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
        <ElTableColumn fixed="right" label="操作" width="280">
          <template #default="{ row }">
            <ElButton link type="primary" @click="openEdit(row)">编辑</ElButton>
            <ElButton link type="warning" @click="openReset(row)">
              重置密码
            </ElButton>
            <ElButton link type="info" @click="disableUser(row)">禁用</ElButton>
            <ElButton link type="danger" @click="deleteUser(row)">
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
              loadUsers();
            }
          "
          @size-change="
            (size) => {
              page.s = size;
              page.o = 0;
              loadUsers();
            }
          "
        />
      </div>
    </div>

    <ElDialog v-model="editorVisible" :title="title" width="720px">
      <ElForm :model="form" class="grid grid-cols-2 gap-x-4" label-width="90px">
        <ElFormItem label="用户名">
          <ElInput v-model="form.username" />
        </ElFormItem>
        <ElFormItem label="姓名">
          <ElInput v-model="form.realName" />
        </ElFormItem>
        <ElFormItem class="col-span-2" label="头像地址">
          <ElInput v-model="form.avatar" placeholder="https://" />
        </ElFormItem>
        <ElFormItem v-if="!editingId" label="初始密码">
          <ElInput v-model="form.password" show-password />
        </ElFormItem>
        <ElFormItem label="首页">
          <ElInput v-model="form.homePath" />
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
        <ElFormItem class="col-span-2" label="角色">
          <ElSelect v-model="form.roleIds" class="w-full" multiple>
            <ElOption
              v-for="role in roles"
              :key="role.id"
              :label="`${role.name} (${role.code})`"
              :value="role.id"
            />
          </ElSelect>
        </ElFormItem>
      </ElForm>
      <template #footer>
        <ElButton @click="editorVisible = false">取消</ElButton>
        <ElButton type="primary" @click="saveUser">保存</ElButton>
      </template>
    </ElDialog>

    <ElDialog v-model="passwordVisible" title="重置密码" width="420px">
      <ElInput v-model="passwordForm.password" show-password />
      <template #footer>
        <ElButton @click="passwordVisible = false">取消</ElButton>
        <ElButton type="primary" @click="resetPassword">保存</ElButton>
      </template>
    </ElDialog>
  </Page>
</template>
