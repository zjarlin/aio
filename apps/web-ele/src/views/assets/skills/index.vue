<script lang="ts" setup>
import type { FormInstance, FormRules } from 'element-plus';

import { computed, onMounted, reactive, ref } from 'vue';

import { Page } from '@vben/common-ui';
import { IconifyIcon } from '@vben/icons';

import {
  ElButton,
  ElDialog,
  ElForm,
  ElFormItem,
  ElInput,
  ElInputNumber,
  ElMessage,
  ElMessageBox,
  ElOption,
  ElPagination,
  ElSelect,
  ElTable,
  ElTableColumn,
  ElTabPane,
  ElTabs,
  ElTag,
} from 'element-plus';

import {
  skillCreateApi,
  skillDeleteApi,
  skillPageApi,
  type SkillRecord,
  skillSyncSourcesApi,
  skillToggleApi,
  skillUpdateApi,
} from '#/api';

import { formatTime } from '../../system/shared';
import QuickComposer from '../components/quick-composer.vue';
import {
  assetUsageOptions,
  assetUsageText,
  displayTags,
  nextAssetUsageStatus,
  nextAssetUsageText,
  normalizeTags,
} from '../shared';

const loading = ref(false);
const syncing = ref(false);
const skills = ref<SkillRecord[]>([]);
const activeTab = ref<'market' | 'mine'>('market');
const keyword = ref('');
const category = ref('');
const status = ref('');
const page = reactive({ o: 0, s: 10, total: 0 });
const editorVisible = ref(false);
const editingId = ref('');
const formRef = ref<FormInstance>();
const form = reactive({
  category: '',
  description: '',
  name: '',
  prompt: '',
  sortOrder: 0,
  status: 'disabled',
  tags: [] as string[],
});
const formRules: FormRules = {
  name: [{ message: '名称必填', required: true, trigger: 'blur' }],
};

const title = computed(() => (editingId.value ? '编辑技能' : '新增技能'));
const tabMeta = computed(() =>
  activeTab.value === 'mine'
    ? {
        description: '仅展示已加入“我的技能”的技能，名称即技能 ID',
        label: '我的技能',
      }
    : {
        description: '浏览、同步和维护全部本机 / 远程技能定义',
        label: '技能市场',
      },
);
const categories = computed(() => {
  const values = skills.value.flatMap((item) =>
    displayTags(item.category, item.tags),
  );
  return [...new Set(values)];
});

function inferSkillName(content: string) {
  const firstLine = content
    .split('\n')
    .map((line) => line.trim())
    .find(Boolean);
  return firstLine?.replace(/^#+\s*/, '').slice(0, 48) || '未命名技能';
}

function parseTags(content: string) {
  const matches = content.match(/#[\u4E00-\u9FA5\w-]+/g) ?? [];
  return [...new Set(matches.map((tag) => tag.slice(1)))];
}

function sourceLabel(source: SkillRecord['sources'][number]) {
  return source.kind === 'local'
    ? `本机 / ${source.root}`
    : `${source.host} / ${source.root}`;
}

function sourceLabels(row: SkillRecord) {
  return [...new Set(row.sources.map((source) => sourceLabel(source)))];
}

async function loadSkills() {
  loading.value = true;
  try {
    const result = await skillPageApi({
      category: category.value || undefined,
      keyword: keyword.value,
      o: page.o,
      s: page.s,
      status:
        activeTab.value === 'mine' ? 'enabled' : status.value || undefined,
    });
    skills.value = result.d;
    page.total = result.t;
  } finally {
    loading.value = false;
  }
}

function openCreate() {
  editingId.value = '';
  Object.assign(form, {
    category: '',
    description: '',
    name: '',
    prompt: '',
    sortOrder: 0,
    status: 'disabled',
    tags: [],
  });
  editorVisible.value = true;
}

async function quickCreate(content: string, done: (success?: boolean) => void) {
  const name = inferSkillName(content);
  const tags = parseTags(content);
  try {
    await skillCreateApi({
      category: tags[0] || '',
      code: name,
      description: content.split('\n').slice(1, 3).join(' ').slice(0, 120),
      name,
      prompt: content,
      sortOrder: 0,
      status: 'disabled',
      tags,
    });
    await loadSkills();
    done(true);
  } catch {
    done(false);
  }
}

function openEdit(row: SkillRecord) {
  editingId.value = row.id;
  Object.assign(form, {
    category: row.category,
    description: row.description,
    name: row.name,
    prompt: row.prompt,
    sortOrder: row.sortOrder,
    status: row.status,
    tags: displayTags(row.category, row.tags),
  });
  editorVisible.value = true;
}

async function saveSkill() {
  await formRef.value?.validate();
  const tags = normalizeTags(form.tags);
  const name = form.name.trim();
  const input = {
    ...form,
    category: tags[0] || '',
    code: name,
    name,
    tags,
  };
  await (editingId.value
    ? skillUpdateApi({ id: editingId.value, ...input })
    : skillCreateApi(input));
  editorVisible.value = false;
  await loadSkills();
}

async function toggleSkill(row: SkillRecord) {
  const nextStatus = nextAssetUsageStatus(row.status);
  await skillToggleApi(row.id, nextStatus);
  await loadSkills();
}

async function deleteSkill(row: SkillRecord) {
  await ElMessageBox.confirm(`确认删除技能 ${row.name}？`, '删除确认');
  await skillDeleteApi(row.id);
  await loadSkills();
}

async function syncSkills() {
  syncing.value = true;
  try {
    const result = await skillSyncSourcesApi();
    const summary = `扫描 ${result.scanned}，新增 ${result.imported}，更新 ${result.updated}，去重 ${result.deduplicated}，未变更 ${result.unchanged}，跳过 ${result.skipped}`;
    if (result.errors.length > 0) {
      ElMessage.warning(`${summary}，错误 ${result.errors.length}`);
    } else {
      ElMessage.success(summary);
    }
    await loadSkills();
  } finally {
    syncing.value = false;
  }
}

function search() {
  page.o = 0;
  loadSkills();
}

function switchTab() {
  page.o = 0;
  status.value = '';
  loadSkills();
}

onMounted(loadSkills);
</script>

<template>
  <Page description="维护本地技能定义、分类标签和提示词内容" title="技能管理">
    <div class="space-y-3">
      <ElTabs v-model="activeTab" @tab-change="switchTab">
        <ElTabPane label="技能市场" name="market" />
        <ElTabPane label="我的技能" name="mine" />
      </ElTabs>

      <QuickComposer
        v-if="activeTab === 'market'"
        placeholder="写下技能名称和提示词内容，首行会作为技能名称；支持 Markdown 和 #标签"
        submit-text="创建"
        @submit="quickCreate"
      />

      <div class="flex flex-wrap items-center justify-between gap-3">
        <div class="flex flex-wrap items-center gap-2">
          <div class="mr-2 min-w-44">
            <div class="text-sm font-semibold">{{ tabMeta.label }}</div>
            <div class="text-muted-foreground mt-1 text-xs">
              {{ tabMeta.description }}
            </div>
          </div>
          <ElInput
            v-model="keyword"
            class="w-64"
            clearable
            placeholder="名称 / 内容"
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
            v-if="activeTab === 'market'"
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
        <div v-if="activeTab === 'market'" class="flex items-center gap-2">
          <ElButton :loading="syncing" @click="syncSkills">
            <IconifyIcon icon="lucide:refresh-cw" />
            <span>同步本机 / 远程</span>
          </ElButton>
          <ElButton type="primary" @click="openCreate">新增技能</ElButton>
        </div>
      </div>

      <ElTable :data="skills" border size="small" stripe v-loading="loading">
        <ElTableColumn
          label="名称"
          min-width="150"
          prop="name"
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
          min-width="220"
          prop="description"
          show-overflow-tooltip
        />
        <ElTableColumn label="来源" min-width="160">
          <template #default="{ row }">
            <ElTag
              v-for="item in sourceLabels(row)"
              :key="item"
              class="mr-1"
              size="small"
              type="warning"
            >
              {{ item }}
            </ElTag>
          </template>
        </ElTableColumn>
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
            <ElButton link type="warning" @click="toggleSkill(row)">
              {{ nextAssetUsageText(row.status) }}
            </ElButton>
            <ElButton link type="danger" @click="deleteSkill(row)">
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
              loadSkills();
            }
          "
          @size-change="
            (size) => {
              page.s = size;
              page.o = 0;
              loadSkills();
            }
          "
        />
      </div>
    </div>

    <ElDialog v-model="editorVisible" :title="title" width="860px">
      <ElForm
        ref="formRef"
        :model="form"
        :rules="formRules"
        class="grid grid-cols-2 gap-x-4"
        label-width="88px"
      >
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
        <ElFormItem class="col-span-2" label="提示词">
          <ElInput v-model="form.prompt" :rows="10" type="textarea" />
        </ElFormItem>
      </ElForm>
      <template #footer>
        <ElButton @click="editorVisible = false">取消</ElButton>
        <ElButton type="primary" @click="saveSkill">保存</ElButton>
      </template>
    </ElDialog>
  </Page>
</template>
