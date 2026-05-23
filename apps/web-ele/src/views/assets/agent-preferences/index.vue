<script lang="ts" setup>
import type { FormInstance, FormRules } from 'element-plus';

import { computed, onMounted, reactive, ref, watch } from 'vue';

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
  ElTabPane,
  ElTabs,
  ElTag,
} from 'element-plus';

import {
  agentPreferenceCreateApi,
  agentPreferenceDeleteApi,
  agentPreferencePageApi,
  type AgentPreferenceRecord,
  type AgentPreferenceSection,
  agentPreferenceToggleApi,
  agentPreferenceUpdateApi,
} from '#/api';

import { formatTime } from '../../system/shared';
import { displayTags, normalizeTags } from '../shared';

const ruleStatusOptions = [
  { label: '启用', value: 'enabled' },
  { label: '停用', value: 'disabled' },
];

function ruleStatusText(status: string) {
  return status === 'enabled' ? '启用' : '停用';
}

function nextRuleStatus(status: string) {
  return status === 'enabled' ? 'disabled' : 'enabled';
}

function nextRuleStatusText(status: string) {
  return status === 'enabled' ? '停用' : '启用';
}

const sectionOptions: Array<{
  description: string;
  label: string;
  value: AgentPreferenceSection;
}> = [
  {
    description: '沟通风格、交付节奏、验证标准和长期协作习惯',
    label: '个人偏好',
    value: 'personal',
  },
  {
    description: 'Rust、Java、Kotlin、前端等技术领域里的稳定偏好',
    label: '领域偏好',
    value: 'domain',
  },
  {
    description: '适合公开复用、可放进团队或开源上下文的通用原则',
    label: '公开偏好',
    value: 'public',
  },
  {
    description: '架构、模块边界、错误处理、配置和 UI 组织模式',
    label: '设计模式',
    value: 'design_patterns',
  },
];

const loading = ref(false);
const items = ref<AgentPreferenceRecord[]>([]);
const keyword = ref('');
const domain = ref('');
const status = ref('');
const activeSection = ref<AgentPreferenceSection>('personal');
const page = reactive({ o: 0, s: 10, total: 0 });
const editorVisible = ref(false);
const editingId = ref('');
const formRef = ref<FormInstance>();
const form = reactive({
  code: '',
  content: '',
  domain: '',
  rationale: '',
  section: 'personal' as AgentPreferenceSection,
  sortOrder: 0,
  status: 'enabled',
  tags: [] as string[],
  title: '',
});
const formRules: FormRules = {
  code: [{ message: '编码必填', required: true, trigger: 'blur' }],
  section: [{ message: '分区必选', required: true, trigger: 'change' }],
  title: [{ message: '标题必填', required: true, trigger: 'blur' }],
};

const sectionMeta = computed(
  () =>
    sectionOptions.find((item) => item.value === activeSection.value) ??
    sectionOptions[0]!,
);
const dialogTitle = computed(() =>
  editingId.value ? '编辑规则项' : '新增规则项',
);
const activeRows = computed(() =>
  items.value.filter((item) => item.status === 'enabled'),
);
const domains = computed(() => {
  const values = items.value.flatMap((item) =>
    normalizeTags([item.domain, ...item.tags]),
  );
  return [...new Set(values)];
});
const markdownPreview = computed(() => {
  const rows = activeRows.value;
  if (rows.length === 0) {
    return '当前分区暂无启用规则。';
  }

  return rows
    .map((item) => {
      const scope = item.domain ? ` (${item.domain})` : '';
      return `- ${item.title}${scope}: ${item.content || item.rationale || '暂无内容'}`;
    })
    .join('\n');
});
const enabledCount = computed(() => activeRows.value.length);
const disabledCount = computed(
  () => items.value.filter((item) => item.status !== 'enabled').length,
);

function sectionLabel(section: AgentPreferenceSection) {
  return (
    sectionOptions.find((item) => item.value === section)?.label ?? section
  );
}

function slugify(value: string) {
  const slug = value
    .trim()
    .toLowerCase()
    .replaceAll(/[^a-z0-9]+/g, '-')
    .replaceAll(/^-+|-+$/g, '');
  return `agent-${activeSection.value}-${slug || 'rule'}-${Date.now().toString(36)}`;
}

async function loadItems() {
  loading.value = true;
  try {
    const result = await agentPreferencePageApi({
      domain: domain.value || undefined,
      keyword: keyword.value,
      o: page.o,
      s: page.s,
      section: activeSection.value,
      status: status.value || undefined,
    });
    items.value = result.d;
    page.total = result.t;
  } finally {
    loading.value = false;
  }
}

function resetForm(section = activeSection.value) {
  Object.assign(form, {
    code: '',
    content: '',
    domain: section === 'domain' ? domain.value || 'rust' : '',
    rationale: '',
    section,
    sortOrder: 0,
    status: 'enabled',
    tags: [],
    title: '',
  });
}

function openCreate() {
  editingId.value = '';
  resetForm();
  editorVisible.value = true;
}

function openEdit(row: AgentPreferenceRecord) {
  editingId.value = row.id;
  Object.assign(form, {
    code: row.code,
    content: row.content,
    domain: row.domain,
    rationale: row.rationale,
    section: row.section,
    sortOrder: row.sortOrder,
    status: row.status,
    tags: displayTags(row.domain, row.tags),
    title: row.title,
  });
  editorVisible.value = true;
}

async function saveItem() {
  await formRef.value?.validate();
  const tags = normalizeTags(form.tags);
  const title = form.title.trim();
  const input = {
    code: form.code.trim() || slugify(title),
    content: form.content,
    domain: form.domain.trim(),
    rationale: form.rationale,
    section: form.section,
    sortOrder: form.sortOrder,
    status: form.status,
    tags,
    title,
  };

  await (editingId.value
    ? agentPreferenceUpdateApi({ id: editingId.value, ...input })
    : agentPreferenceCreateApi(input));
  editorVisible.value = false;
  if (activeSection.value !== form.section) {
    activeSection.value = form.section;
    return;
  }
  await loadItems();
}

async function toggleItem(row: AgentPreferenceRecord) {
  const nextStatus = nextRuleStatus(row.status);
  await agentPreferenceToggleApi(row.id, nextStatus);
  await loadItems();
}

async function deleteItem(row: AgentPreferenceRecord) {
  await ElMessageBox.confirm(`确认删除规则项 ${row.title}？`, '删除确认');
  await agentPreferenceDeleteApi(row.id);
  await loadItems();
}

function search() {
  page.o = 0;
  loadItems();
}

watch(activeSection, () => {
  page.o = 0;
  domain.value = '';
  status.value = '';
  void loadItems();
});

onMounted(loadItems);
</script>

<template>
  <Page
    description="按个人偏好、领域偏好、公开偏好和设计模式维护全局 AGENTS.md 规则项"
    title="AGENTS.md 管理"
  >
    <div class="space-y-4">
      <ElTabs v-model="activeSection">
        <ElTabPane
          v-for="section in sectionOptions"
          :key="section.value"
          :label="section.label"
          :name="section.value"
        />
      </ElTabs>

      <div class="grid grid-cols-1 gap-4 xl:grid-cols-[1fr_360px]">
        <section class="min-w-0 space-y-3">
          <div
            class="border-border bg-background flex flex-wrap items-center justify-between gap-3 rounded border px-3 py-3"
          >
            <div class="min-w-64">
              <div class="text-sm font-semibold">{{ sectionMeta.label }}</div>
              <div class="text-muted-foreground mt-1 text-xs">
                {{ sectionMeta.description }}
              </div>
            </div>
            <div class="flex flex-wrap items-center gap-2">
              <ElInput
                v-model="keyword"
                class="w-56"
                clearable
                placeholder="标题 / 编码 / 内容"
                @keyup.enter="search"
              />
              <ElSelect
                v-model="domain"
                allow-create
                class="w-36"
                clearable
                filterable
                placeholder="领域"
              >
                <ElOption
                  v-for="item in domains"
                  :key="item"
                  :label="item"
                  :value="item"
                />
              </ElSelect>
              <ElSelect
                v-model="status"
                class="w-32"
                clearable
                placeholder="状态"
              >
                <ElOption
                  v-for="item in ruleStatusOptions"
                  :key="item.value"
                  :label="item.label"
                  :value="item.value"
                />
              </ElSelect>
              <ElButton @click="search">查询</ElButton>
              <ElButton type="primary" @click="openCreate">新增规则</ElButton>
            </div>
          </div>

          <ElTable :data="items" border size="small" stripe v-loading="loading">
            <ElTableColumn label="标题" min-width="180" show-overflow-tooltip>
              <template #default="{ row }">
                <div class="font-medium">{{ row.title }}</div>
                <div class="text-muted-foreground text-xs">{{ row.code }}</div>
              </template>
            </ElTableColumn>
            <ElTableColumn label="分区" width="110">
              <template #default="{ row }">
                {{ sectionLabel(row.section) }}
              </template>
            </ElTableColumn>
            <ElTableColumn label="领域 / tag" min-width="170">
              <template #default="{ row }">
                <ElTag
                  v-for="tag in displayTags(row.domain, row.tags)"
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
              label="规则内容"
              min-width="260"
              prop="content"
              show-overflow-tooltip
            />
            <ElTableColumn
              label="原因"
              min-width="180"
              prop="rationale"
              show-overflow-tooltip
            />
            <ElTableColumn label="排序" prop="sortOrder" width="76" />
            <ElTableColumn label="状态" width="90">
              <template #default="{ row }">
                <ElTag :type="row.status === 'enabled' ? 'success' : 'info'">
                  {{ ruleStatusText(row.status) }}
                </ElTag>
              </template>
            </ElTableColumn>
            <ElTableColumn label="更新时间" width="168">
              <template #default="{ row }">
                {{ formatTime(row.updatedAt) }}
              </template>
            </ElTableColumn>
            <ElTableColumn fixed="right" label="操作" width="210">
              <template #default="{ row }">
                <ElButton link type="primary" @click="openEdit(row)">
                  编辑
                </ElButton>
                <ElButton link type="warning" @click="toggleItem(row)">
                  {{ nextRuleStatusText(row.status) }}
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
        </section>

        <aside class="space-y-3">
          <div
            class="border-border bg-background grid grid-cols-3 gap-2 rounded border p-3"
          >
            <div>
              <div class="text-muted-foreground text-xs">总数</div>
              <div class="mt-1 text-lg font-semibold">{{ page.total }}</div>
            </div>
            <div>
              <div class="text-muted-foreground text-xs">当前页启用</div>
              <div class="mt-1 text-lg font-semibold">{{ enabledCount }}</div>
            </div>
            <div>
              <div class="text-muted-foreground text-xs">当前页停用</div>
              <div class="mt-1 text-lg font-semibold">{{ disabledCount }}</div>
            </div>
          </div>

          <div class="border-border bg-background rounded border p-3">
            <div class="flex items-center justify-between gap-2">
              <div class="text-sm font-semibold">当前页预览</div>
              <ElButton link type="primary" @click="openCreate">
                快速新增
              </ElButton>
            </div>
            <pre class="markdown-preview">{{ markdownPreview }}</pre>
          </div>
        </aside>
      </div>
    </div>

    <ElDialog v-model="editorVisible" :title="dialogTitle" width="920px">
      <ElForm
        ref="formRef"
        :model="form"
        :rules="formRules"
        class="grid grid-cols-2 gap-x-4"
        label-width="92px"
      >
        <ElFormItem label="分区" prop="section">
          <ElSelect v-model="form.section" class="w-full">
            <ElOption
              v-for="item in sectionOptions"
              :key="item.value"
              :label="item.label"
              :value="item.value"
            />
          </ElSelect>
        </ElFormItem>
        <ElFormItem label="领域">
          <ElInput v-model="form.domain" placeholder="rust / java / public" />
        </ElFormItem>
        <ElFormItem label="编码" prop="code">
          <ElInput v-model="form.code" />
        </ElFormItem>
        <ElFormItem label="标题" prop="title">
          <ElInput v-model="form.title" />
        </ElFormItem>
        <ElFormItem label="状态">
          <ElSelect v-model="form.status" class="w-full">
            <ElOption
              v-for="item in ruleStatusOptions"
              :key="item.value"
              :label="item.label"
              :value="item.value"
            />
          </ElSelect>
        </ElFormItem>
        <ElFormItem label="排序">
          <ElInputNumber v-model="form.sortOrder" class="w-full" />
        </ElFormItem>
        <ElFormItem class="col-span-2" label="tag">
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
        <ElFormItem class="col-span-2" label="规则正文">
          <ElInput
            v-model="form.content"
            :rows="7"
            placeholder="一条可直接放进 AGENTS.md 的规则"
            type="textarea"
          />
        </ElFormItem>
        <ElFormItem class="col-span-2" label="原因">
          <ElInput
            v-model="form.rationale"
            :rows="3"
            placeholder="记录为什么需要这条规则，便于以后清理或公开化"
            type="textarea"
          />
        </ElFormItem>
      </ElForm>
      <template #footer>
        <ElButton @click="editorVisible = false">取消</ElButton>
        <ElButton type="primary" @click="saveItem">保存</ElButton>
      </template>
    </ElDialog>
  </Page>
</template>

<style scoped>
.markdown-preview {
  max-height: 520px;
  padding: 12px;
  margin-top: 12px;
  overflow: auto;
  font-size: 12px;
  line-height: 20px;
  white-space: pre-wrap;
  background: hsl(var(--muted));
  border-radius: 6px;
}
</style>
