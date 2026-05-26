<script lang="ts" setup>
import { computed, onMounted, reactive, ref } from 'vue';

import { Page } from '@vben/common-ui';

import {
  ElButton,
  ElDialog,
  ElForm,
  ElFormItem,
  ElInput,
  ElMessage,
  ElOption,
  ElPagination,
  ElSelect,
  ElTable,
  ElTableColumn,
  ElTag,
} from 'element-plus';

import {
  type ComputerRecord,
  dotfileComputerListApi,
  dotfileComputerUpsertApi,
  dotfileFusionListApi,
  type DotfileFusionRecord,
  dotfileMetadataImportApi,
  dotfileScanComputerApi,
  dotfileSnapshotPageApi,
  type DotfileSnapshotRecord,
} from '#/api';

import { formatTime } from '../../system/shared';

const loading = ref(false);
const scanning = ref(false);
const importing = ref(false);
const computers = ref<ComputerRecord[]>([]);
const snapshots = ref<DotfileSnapshotRecord[]>([]);
const fusionRows = ref<DotfileFusionRecord[]>([]);
const keyword = ref('');
const computerId = ref('');
const viewMode = ref<'fusion' | 'snapshots'>('fusion');
const page = reactive({ o: 0, s: 10, total: 0 });
const computerDialogVisible = ref(false);
const computerForm = reactive({
  host: '',
  kind: 'ssh',
  name: '',
  site: '',
  username: '',
});

const selectedComputerName = computed(() => {
  return (
    computers.value.find((computer) => computer.id === computerId.value)
      ?.name || ''
  );
});

async function loadComputers() {
  computers.value = await dotfileComputerListApi();
  if (!computerId.value && computers.value.length > 0) {
    computerId.value = computers.value[0]!.id;
  }
}

async function loadSnapshots() {
  const result = await dotfileSnapshotPageApi({
    computerId: computerId.value || undefined,
    keyword: keyword.value,
    o: page.o,
    s: page.s,
  });
  snapshots.value = result.d;
  page.total = result.t;
}

async function loadFusion() {
  fusionRows.value = await dotfileFusionListApi();
}

async function loadPage() {
  loading.value = true;
  try {
    await loadComputers();
    await Promise.all([loadSnapshots(), loadFusion()]);
  } finally {
    loading.value = false;
  }
}

async function importMetadata() {
  importing.value = true;
  try {
    const result = await dotfileMetadataImportApi();
    const envCount = result.envVars ?? result.env_vars ?? 0;
    const summary = `导入 dotfiles ${result.dotfiles}，环境变量 ${envCount}`;
    if (result.errors.length > 0) {
      ElMessage.warning(`${summary}，错误 ${result.errors.length}`);
    } else {
      ElMessage.success(summary);
    }
    await loadPage();
  } finally {
    importing.value = false;
  }
}

async function scanComputer(targetId = computerId.value) {
  scanning.value = true;
  try {
    const result = await dotfileScanComputerApi(targetId || undefined);
    const summary = `扫描 ${result.scanned}，新增 ${result.inserted}，更新 ${result.updated}，未变更 ${result.unchanged}，缺失 ${result.deleted}`;
    if (result.errors.length > 0) {
      ElMessage.warning(`${summary}，错误 ${result.errors.length}`);
    } else {
      ElMessage.success(summary);
    }
    await loadPage();
  } finally {
    scanning.value = false;
  }
}

function search() {
  page.o = 0;
  loadSnapshots();
}

function openComputerDialog() {
  Object.assign(computerForm, {
    host: '',
    kind: 'ssh',
    name: '',
    site: '',
    username: '',
  });
  computerDialogVisible.value = true;
}

async function saveComputer() {
  await dotfileComputerUpsertApi({
    host: computerForm.host,
    kind: computerForm.kind,
    name: computerForm.name,
    site: computerForm.site,
    username: computerForm.username,
  });
  computerDialogVisible.value = false;
  await loadComputers();
}

function statusType(row: DotfileFusionRecord) {
  if (row.missingCount > 0) return 'warning';
  if (row.variantCount > 1) return 'danger';
  return 'success';
}

function statusText(row: DotfileFusionRecord) {
  if (row.missingCount > 0) return '缺失';
  if (row.variantCount > 1) return '冲突';
  return '一致';
}

onMounted(loadPage);
</script>

<template>
  <Page
    description="按登录用户绑定主机，扫描本机/远程家目录 dotfiles，并融合多台计算机的配置状态"
    title="dotfiles 管理"
  >
    <div class="space-y-3">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div class="flex flex-wrap items-center gap-2">
          <ElSelect v-model="viewMode" class="w-32" @change="loadPage">
            <ElOption label="融合视图" value="fusion" />
            <ElOption label="扫描明细" value="snapshots" />
          </ElSelect>
          <ElSelect
            v-model="computerId"
            class="w-48"
            clearable
            filterable
            placeholder="主机"
            @change="search"
          >
            <ElOption
              v-for="computer in computers"
              :key="computer.id"
              :label="`${computer.name} / ${computer.kind}`"
              :value="computer.id"
            />
          </ElSelect>
          <ElInput
            v-model="keyword"
            class="w-64"
            clearable
            placeholder="路径 / 内容 / hash"
            @keyup.enter="search"
          />
          <ElButton @click="search">查询</ElButton>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <ElButton :loading="importing" @click="importMetadata">
            导入 Excel 元数据
          </ElButton>
          <ElButton :loading="scanning" @click="scanComputer()">
            扫描{{ selectedComputerName ? ` ${selectedComputerName}` : '' }}
          </ElButton>
          <ElButton type="primary" @click="openComputerDialog">
            新增远程主机
          </ElButton>
        </div>
      </div>

      <ElTable
        v-if="viewMode === 'fusion'"
        :data="fusionRows"
        border
        size="small"
        stripe
        v-loading="loading"
      >
        <ElTableColumn
          label="部署目标"
          min-width="240"
          prop="deployTarget"
          show-overflow-tooltip
        />
        <ElTableColumn
          label="说明"
          min-width="180"
          prop="description"
          show-overflow-tooltip
        />
        <ElTableColumn label="状态" width="100">
          <template #default="{ row }">
            <ElTag :type="statusType(row)">
              {{ statusText(row) }}
            </ElTag>
          </template>
        </ElTableColumn>
        <ElTableColumn label="覆盖主机" prop="computerCount" width="100" />
        <ElTableColumn label="内容版本" prop="variantCount" width="100" />
        <ElTableColumn label="缺失主机" prop="missingCount" width="100" />
        <ElTableColumn label="最近扫描" width="180">
          <template #default="{ row }">
            {{ formatTime(row.latestScannedAt) }}
          </template>
        </ElTableColumn>
      </ElTable>

      <ElTable
        v-else
        :data="snapshots"
        border
        size="small"
        stripe
        v-loading="loading"
      >
        <ElTableColumn
          label="路径"
          min-width="260"
          prop="path"
          show-overflow-tooltip
        />
        <ElTableColumn label="类型" prop="itemType" width="90" />
        <ElTableColumn label="状态" width="90">
          <template #default="{ row }">
            <ElTag :type="row.status === 'tracked' ? 'success' : 'warning'">
              {{ row.status === 'tracked' ? '存在' : '缺失' }}
            </ElTag>
          </template>
        </ElTableColumn>
        <ElTableColumn label="大小" prop="size" width="100" />
        <ElTableColumn
          label="hash"
          min-width="180"
          prop="contentHash"
          show-overflow-tooltip
        />
        <ElTableColumn
          label="预览"
          min-width="220"
          prop="preview"
          show-overflow-tooltip
        />
        <ElTableColumn label="扫描时间" width="180">
          <template #default="{ row }">
            {{ formatTime(row.scannedAt) }}
          </template>
        </ElTableColumn>
      </ElTable>

      <div v-if="viewMode === 'snapshots'" class="flex justify-end">
        <ElPagination
          :page-size="page.s"
          :total="page.total"
          layout="total, sizes, prev, pager, next"
          @current-change="
            (current) => {
              page.o = (current - 1) * page.s;
              loadSnapshots();
            }
          "
          @size-change="
            (size) => {
              page.s = size;
              page.o = 0;
              loadSnapshots();
            }
          "
        />
      </div>
    </div>

    <ElDialog
      v-model="computerDialogVisible"
      title="新增远程主机"
      width="520px"
    >
      <ElForm :model="computerForm" label-width="88px">
        <ElFormItem label="名称">
          <ElInput v-model="computerForm.name" placeholder="raw.addzero.site" />
        </ElFormItem>
        <ElFormItem label="SSH Host">
          <ElInput v-model="computerForm.host" placeholder="raw.addzero.site" />
        </ElFormItem>
        <ElFormItem label="用户名">
          <ElInput v-model="computerForm.username" placeholder="zjarlin" />
        </ElFormItem>
        <ElFormItem label="场景">
          <ElInput v-model="computerForm.site" placeholder="home / office" />
        </ElFormItem>
      </ElForm>
      <template #footer>
        <ElButton @click="computerDialogVisible = false">取消</ElButton>
        <ElButton type="primary" @click="saveComputer">保存</ElButton>
      </template>
    </ElDialog>
  </Page>
</template>
