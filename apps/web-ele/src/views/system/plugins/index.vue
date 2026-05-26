<script lang="ts" setup>
import type {
  AppRuntimeLifecycleRecord,
  AppRuntimeReloadInput,
  AppRuntimeSessionInput,
  AppRuntimeSnapshot,
  AppRuntimeStartInput,
  AppRuntimeStopInput,
  AppRuntimeWorkspaceInput,
  ChildCapabilityApprovalRecord,
  PermissionApprovalRecord,
  PermissionConsentRecord,
  PermissionDecisionRecord,
  PlatformEventRecord,
  PluginPublishGateReport,
  PluginRegistryLocalState,
  PluginRegistryRollbackResult,
  PluginRegistrySnapshot,
  PluginUiRenderSchema,
  PluginUiTreeNode,
  PluginVerificationReport,
} from '@vben/types';

import { computed, onMounted, reactive, ref } from 'vue';

import { Page } from '@vben/common-ui';

import {
  ElAlert,
  ElButton,
  ElInput,
  ElMessage,
  ElMessageBox,
  ElOption,
  ElSelect,
  ElSwitch,
  ElTable,
  ElTableColumn,
  ElTag,
} from 'element-plus';

import {
  appRuntimeReloadApi,
  appRuntimeSessionApi,
  appRuntimeSnapshotApi,
  appRuntimeStartApi,
  appRuntimeStopApi,
  appRuntimeWorkspaceApi,
  eventBusSnapshotApi,
  permissionApprovalApproveApi,
  permissionApprovalDenyApi,
  permissionApprovalListApi,
  permissionAuditLogApi,
  permissionConsentGrantApi,
  permissionConsentListApi,
  permissionConsentRevokeApi,
  pluginChildCapabilityApproveApi,
  pluginChildCapabilityRevokeApi,
  pluginPublishGateApi,
  pluginRegistryLocalStateApi,
  pluginRegistryReloadApi,
  pluginRegistryRollbackApi,
  pluginRegistrySnapshotApi,
  pluginVerifyDraftApi,
} from '#/api/core';

import { formatTime } from '../shared';
import PluginViewRenderer from './components/plugin-view-renderer.vue';

const loading = ref(false);
const runtimeSnapshot = ref<AppRuntimeSnapshot | null>(null);
const eventBusRecords = ref<PlatformEventRecord[]>([]);
const permissionAuditRecords = ref<PermissionDecisionRecord[]>([]);
const permissionApprovalRecords = ref<PermissionApprovalRecord[]>([]);
const permissionConsentRecords = ref<PermissionConsentRecord[]>([]);
const localState = ref<null | PluginRegistryLocalState>(null);
const snapshot = ref<null | PluginRegistrySnapshot>(null);
const approvalDecisionKey = ref('');
const childCapabilityDecisionKey = ref('');
const consentGrantLoading = ref(false);
const consentRevokeKey = ref('');
const consentGrantForm = reactive({
  capability: '',
  reason: 'manual consent from plugin registry',
  scope: '*',
  sourceId: '',
  sourceKind: 'plugin',
});
const verifyDraftLoading = ref(false);
const verifyDraftOutputDir = ref('');
const verifyDraftSourcePath = ref('');
const verifyDraftWrite = ref(true);
const verificationReport = ref<null | PluginVerificationReport>(null);
const publishGateLoading = ref(false);
const publishGateSourcePath = ref('');
const publishGateWrite = ref(true);
const publishGateReport = ref<null | PluginPublishGateReport>(null);
const rollbackContentHash = ref('');
const rollbackId = ref('');
const rollbackLoading = ref(false);
const rollbackResult = ref<null | PluginRegistryRollbackResult>(null);
const runtimeActionLoading = ref('');
const runtimeStartForm = reactive<AppRuntimeStartInput>({
  mode: 'desktop',
  reason: 'manual runtime start',
  sessionId: '',
  workspace: '',
});
const runtimeReloadForm = reactive<AppRuntimeReloadInput>({
  reason: 'manual runtime reload',
  sessionId: '',
  workspace: '',
});
const runtimeWorkspaceForm = reactive<AppRuntimeWorkspaceInput>({
  reason: 'manual runtime workspace change',
  workspace: '',
});
const runtimeSessionForm = reactive<AppRuntimeSessionInput>({
  reason: 'manual runtime session change',
  sessionId: '',
});
const runtimeStopForm = reactive<AppRuntimeStopInput>({
  reason: 'manual runtime stop',
});

async function loadSnapshot() {
  loading.value = true;
  try {
    const [
      runtime,
      registrySnapshot,
      registryState,
      eventRecords,
      permissionRecords,
      approvalRecords,
      consentRecords,
    ] = await Promise.all([
      appRuntimeSnapshotApi(),
      pluginRegistrySnapshotApi(),
      pluginRegistryLocalStateApi(),
      eventBusSnapshotApi({ limit: 50 }),
      permissionAuditLogApi(),
      permissionApprovalListApi(),
      permissionConsentListApi(),
    ]);
    runtimeSnapshot.value = runtime;
    snapshot.value = registrySnapshot;
    localState.value = registryState;
    eventBusRecords.value = eventRecords;
    permissionAuditRecords.value = permissionRecords;
    permissionApprovalRecords.value = approvalRecords;
    permissionConsentRecords.value = consentRecords;
  } finally {
    loading.value = false;
  }
}

onMounted(loadSnapshot);

async function reloadRegistry() {
  await pluginRegistryReloadApi();
  await loadSnapshot();
}

function formatPayload(payload: unknown) {
  try {
    return JSON.stringify(payload);
  } catch {
    return `${payload}`;
  }
}

function eventTone(kind: PlatformEventRecord['kind']) {
  if (kind === 'error') {
    return 'danger';
  }
  if (kind === 'reply') {
    return 'success';
  }
  if (kind === 'request') {
    return 'warning';
  }
  return 'info';
}

function countItems(value: unknown) {
  return Array.isArray(value) ? value.length : 0;
}

function runtimeActionKey(action: string) {
  return action;
}

const runtimeUpdatedAtText = computed(() =>
  runtimeSnapshot.value?.updatedAt
    ? formatTime(runtimeSnapshot.value.updatedAt)
    : '-',
);

async function withRuntimeAction(
  action: string,
  handler: () => Promise<AppRuntimeLifecycleRecord>,
) {
  runtimeActionLoading.value = action;
  try {
    await handler();
    await loadSnapshot();
  } finally {
    runtimeActionLoading.value = '';
  }
}

async function startRuntime() {
  await withRuntimeAction('start', () =>
    appRuntimeStartApi({
      ...runtimeStartForm,
      mode: runtimeStartForm.mode?.trim() || 'desktop',
      reason: runtimeStartForm.reason?.trim() || undefined,
      sessionId: runtimeStartForm.sessionId?.trim() || undefined,
      workspace: runtimeStartForm.workspace?.trim() || undefined,
    }),
  );
}

async function stopRuntime() {
  await withRuntimeAction('stop', () =>
    appRuntimeStopApi({
      reason: runtimeStopForm.reason?.trim() || undefined,
    }),
  );
}

async function reloadRuntime() {
  await withRuntimeAction('reload', () =>
    appRuntimeReloadApi({
      ...runtimeReloadForm,
      reason: runtimeReloadForm.reason?.trim() || undefined,
      sessionId: runtimeReloadForm.sessionId?.trim() || undefined,
      workspace: runtimeReloadForm.workspace?.trim() || undefined,
    }),
  );
}

async function updateRuntimeWorkspace() {
  const workspace = runtimeWorkspaceForm.workspace.trim();
  if (!workspace) {
    return;
  }
  await withRuntimeAction('workspace', () =>
    appRuntimeWorkspaceApi({
      reason: runtimeWorkspaceForm.reason?.trim() || undefined,
      workspace,
    }),
  );
}

async function updateRuntimeSession() {
  const sessionId = runtimeSessionForm.sessionId.trim();
  if (!sessionId) {
    return;
  }
  await withRuntimeAction('session', () =>
    appRuntimeSessionApi({
      reason: runtimeSessionForm.reason?.trim() || undefined,
      sessionId,
    }),
  );
}

function normalizeConsentScope(scope?: string) {
  return scope?.trim() || '*';
}

function consentRowKey(row: PermissionConsentRecord) {
  return [row.sourceId, row.capability, row.scope].join('::');
}

function approvalRowKey(row: PermissionApprovalRecord) {
  return row.id;
}

function childCapabilityKey(
  parentPluginId: string,
  childPluginId: string,
  capability: string,
) {
  return [parentPluginId, childPluginId, capability].join('::');
}

function childCapabilityApprovalKey(row: ChildCapabilityApprovalRecord) {
  return childCapabilityKey(
    row.parentPluginId,
    row.childPluginId,
    row.capability,
  );
}

function fillConsentFromCapability(
  row: PluginRegistrySnapshot['capabilities'][number],
) {
  Object.assign(consentGrantForm, {
    capability: row.id,
    reason: row.reason || `manifest capability ${row.id}`,
    scope: normalizeConsentScope(row.scope),
    sourceId: row.sourceId,
    sourceKind: row.sourceKind,
  });
}

function fillConsentFromDecision(row: PermissionDecisionRecord) {
  Object.assign(consentGrantForm, {
    capability: row.capability,
    reason: `audit decision ${row.decision}: ${row.reason}`,
    scope: normalizeConsentScope(row.scope),
    sourceId: row.sourceId,
    sourceKind: row.sourceKind,
  });
}

async function grantConsent() {
  const sourceId = consentGrantForm.sourceId.trim();
  const capability = consentGrantForm.capability.trim();
  if (!sourceId || !capability) {
    ElMessage.warning('sourceId 和 capability 必填');
    return;
  }

  consentGrantLoading.value = true;
  try {
    await permissionConsentGrantApi({
      capability,
      reason: consentGrantForm.reason.trim(),
      scope: normalizeConsentScope(consentGrantForm.scope),
      sourceId,
      sourceKind: consentGrantForm.sourceKind.trim() || 'plugin',
    });
    ElMessage.success('已写入权限同意');
    await loadSnapshot();
  } finally {
    consentGrantLoading.value = false;
  }
}

async function revokeConsent(row: PermissionConsentRecord) {
  await ElMessageBox.confirm(
    `确认撤销 ${row.sourceId} / ${row.capability} / ${row.scope}？`,
    '撤销权限同意',
  );
  consentRevokeKey.value = consentRowKey(row);
  try {
    await permissionConsentRevokeApi({
      capability: row.capability,
      scope: row.scope,
      sourceId: row.sourceId,
    });
    ElMessage.success('已撤销权限同意');
    await loadSnapshot();
  } finally {
    consentRevokeKey.value = '';
  }
}

async function approveApproval(row: PermissionApprovalRecord) {
  await ElMessageBox.confirm(
    `确认通过 ${row.sourceId} / ${row.capability} / ${row.scope}？`,
    '通过权限审批',
  );
  approvalDecisionKey.value = approvalRowKey(row);
  try {
    await permissionApprovalApproveApi({
      id: row.id,
      reason: `approved from registry UI for ${row.capability}`,
    });
    ElMessage.success('已通过审批并写入权限同意');
    await loadSnapshot();
  } finally {
    approvalDecisionKey.value = '';
  }
}

async function denyApproval(row: PermissionApprovalRecord) {
  await ElMessageBox.confirm(
    `确认拒绝 ${row.sourceId} / ${row.capability} / ${row.scope}？`,
    '拒绝权限审批',
    { type: 'warning' },
  );
  approvalDecisionKey.value = approvalRowKey(row);
  try {
    await permissionApprovalDenyApi({
      id: row.id,
      reason: `denied from registry UI for ${row.capability}`,
    });
    ElMessage.success('已拒绝审批');
    await loadSnapshot();
  } finally {
    approvalDecisionKey.value = '';
  }
}

async function approveChildCapability(
  row: PluginRegistrySnapshot['extensionTree']['nodes'][number],
  capability: string,
) {
  const parentPluginId = row.parentPluginId || '';
  if (!parentPluginId) {
    ElMessage.warning('只有子插件能力升级需要审批');
    return;
  }
  await ElMessageBox.confirm(
    `确认批准 ${row.pluginId} 请求 ${capability}？`,
    '批准子插件能力',
  );
  childCapabilityDecisionKey.value = childCapabilityKey(
    parentPluginId,
    row.pluginId,
    capability,
  );
  try {
    await pluginChildCapabilityApproveApi({
      capability,
      childPluginId: row.pluginId,
      parentPluginId,
      reason: `approved from registry UI for ${row.pluginId}/${capability}`,
    });
    ElMessage.success('已批准子插件能力');
    await loadSnapshot();
  } finally {
    childCapabilityDecisionKey.value = '';
  }
}

async function revokeChildCapability(row: ChildCapabilityApprovalRecord) {
  await ElMessageBox.confirm(
    `确认撤销 ${row.childPluginId} / ${row.capability} 的批准？`,
    '撤销子插件能力批准',
    { type: 'warning' },
  );
  childCapabilityDecisionKey.value = childCapabilityApprovalKey(row);
  try {
    await pluginChildCapabilityRevokeApi({
      capability: row.capability,
      childPluginId: row.childPluginId,
      parentPluginId: row.parentPluginId,
      reason: `revoked from registry UI for ${row.childPluginId}/${row.capability}`,
    });
    ElMessage.success('已撤销子插件能力批准');
    await loadSnapshot();
  } finally {
    childCapabilityDecisionKey.value = '';
  }
}

async function verifyDraft() {
  const sourcePath = verifyDraftSourcePath.value.trim();
  if (!sourcePath) {
    return;
  }
  verifyDraftLoading.value = true;
  try {
    verificationReport.value = await pluginVerifyDraftApi({
      outputDir: verifyDraftOutputDir.value.trim() || undefined,
      sourcePath,
      write: verifyDraftWrite.value,
    });
  } finally {
    verifyDraftLoading.value = false;
  }
}

async function runPublishGate() {
  const sourcePath = publishGateSourcePath.value.trim();
  if (!sourcePath) {
    return;
  }
  publishGateLoading.value = true;
  try {
    publishGateReport.value = await pluginPublishGateApi(
      sourcePath,
      publishGateWrite.value,
    );
    if (!verifyDraftSourcePath.value.trim()) {
      verifyDraftSourcePath.value = sourcePath;
    }
    await loadSnapshot();
  } finally {
    publishGateLoading.value = false;
  }
}

async function rollbackPlugin(id?: string, contentHash?: string) {
  const pluginId = (id || rollbackId.value).trim();
  if (!pluginId) {
    return;
  }
  rollbackLoading.value = true;
  try {
    rollbackResult.value = await pluginRegistryRollbackApi({
      id: pluginId,
      contentHash:
        (contentHash || rollbackContentHash.value).trim() || undefined,
    });
    rollbackId.value = pluginId;
    await loadSnapshot();
  } finally {
    rollbackLoading.value = false;
  }
}

function buildPluginTree(
  nodes: PluginRegistrySnapshot['extensionTree']['nodes'],
) {
  const treeById = new Map<string, PluginUiTreeNode>();
  const roots: PluginUiTreeNode[] = [];

  for (const node of [...nodes].sort((left, right) => {
    return (
      left.depth - right.depth ||
      left.displayName.localeCompare(right.displayName)
    );
  })) {
    treeById.set(node.pluginId, {
      id: node.pluginId,
      label: node.displayName,
      value: node.parentMount
        ? `${node.kind} · ${node.parentMount}`
        : node.kind,
      children: [],
    });
  }

  for (const node of [...nodes].sort((left, right) => {
    return (
      left.depth - right.depth ||
      left.displayName.localeCompare(right.displayName)
    );
  })) {
    const current = treeById.get(node.pluginId);
    if (!current) {
      continue;
    }
    const parentId = node.parentPluginId;
    if (parentId) {
      const parent = treeById.get(parentId);
      if (parent) {
        parent.children = parent.children || [];
        parent.children.push(current);
        continue;
      }
    }
    roots.push(current);
  }

  return roots;
}

const viewPreviews = computed<PluginUiRenderSchema[]>(() => {
  const plugins = snapshot.value?.plugins || [];
  const systemCapsules = snapshot.value?.systemCapsules || [];
  const capabilities = snapshot.value?.capabilities || [];
  const capabilityProviders = snapshot.value?.capabilityProviders || [];
  const policies = snapshot.value?.policies || [];
  const settings = snapshot.value?.settings || [];
  const resources = snapshot.value?.resources || [];
  const extensionPoints = snapshot.value?.extensionPoints || [];
  const events = snapshot.value?.events || [];
  const tree = snapshot.value?.extensionTree?.nodes || [];
  const installed = localState.value?.installed || [];
  const locks = localState.value?.locks || [];
  const audits = localState.value?.audits || [];
  const history = localState.value?.history || [];
  const firstPlugin = plugins[0];
  const firstSystemCapsule = systemCapsules[0];

  return [
    {
      kind: 'summary-list',
      title: '注册表摘要',
      items: [
        { label: 'Schema', value: snapshot.value?.schemaVersion || '-' },
        { label: '插件', value: `${countItems(plugins)}` },
        { label: '系统胶囊', value: `${countItems(systemCapsules)}` },
        { label: '能力', value: `${countItems(capabilities)}` },
        { label: 'Provider', value: `${countItems(capabilityProviders)}` },
        { label: '设置', value: `${countItems(settings)}` },
        { label: '资源', value: `${countItems(resources)}` },
        { label: '策略', value: `${countItems(policies)}` },
        { label: '扩展点', value: `${countItems(extensionPoints)}` },
        { label: '事件', value: `${countItems(events)}` },
      ],
    },
    {
      kind: 'detail',
      title: '首个插件详情',
      items: firstPlugin
        ? [
            { label: 'ID', value: firstPlugin.id },
            { label: '名称', value: firstPlugin.displayName },
            { label: '类型', value: firstPlugin.kind },
            { label: '版本', value: firstPlugin.version || '-' },
            { label: '父级', value: firstPlugin.parentPluginId || '-' },
            { label: '命令', value: `${countItems(firstPlugin.commands)}` },
            { label: '视图', value: `${countItems(firstPlugin.views)}` },
            { label: '设置', value: `${countItems(firstPlugin.settings)}` },
            { label: '资源', value: `${countItems(firstPlugin.resources)}` },
            { label: '能力', value: `${countItems(firstPlugin.capabilities)}` },
            {
              label: 'Provider',
              value: `${countItems(firstPlugin.capabilityProviders)}`,
            },
          ]
        : [{ label: '状态', value: '暂无插件快照' }],
    },
    {
      kind: 'form',
      title: '本地 registry',
      fields: [
        {
          label: '安装',
          value: `${countItems(installed)}`,
          hint: 'installed.json',
        },
        { label: '锁定', value: `${countItems(locks)}`, hint: 'lock.json' },
        {
          label: '审计',
          value: `${countItems(audits)}`,
          hint: 'audit.jsonl',
        },
        {
          label: '历史版本',
          value: `${countItems(history)}`,
          hint: 'history/<plugin>/<hash>',
        },
        {
          label: '系统胶囊',
          value: `${countItems(systemCapsules)}`,
          hint: firstSystemCapsule
            ? firstSystemCapsule.displayName
            : '暂无系统胶囊',
        },
      ],
      submitLabel: '重新加载',
    },
    {
      kind: 'table',
      title: '插件表',
      columns: [
        { key: 'id', label: 'ID', width: 220 },
        { key: 'displayName', label: '名称', width: 180 },
        { key: 'kind', label: '类型', width: 120 },
        { key: 'version', label: '版本', width: 100 },
        { key: 'commands', label: '命令', width: 90, align: 'center' },
        { key: 'views', label: '视图', width: 90, align: 'center' },
        { key: 'settings', label: '设置', width: 90, align: 'center' },
        { key: 'resources', label: '资源', width: 90, align: 'center' },
        { key: 'capabilities', label: '能力', width: 90, align: 'center' },
        { key: 'providers', label: 'Provider', width: 110, align: 'center' },
      ],
      rows: plugins.slice(0, 8).map((plugin) => ({
        id: plugin.id,
        displayName: plugin.displayName,
        kind: plugin.kind,
        version: plugin.version || '-',
        commands: countItems(plugin.commands),
        views: countItems(plugin.views),
        settings: countItems(plugin.settings),
        resources: countItems(plugin.resources),
        capabilities: countItems(plugin.capabilities),
        providers: countItems(plugin.capabilityProviders),
      })),
    },
    {
      kind: 'tree',
      title: '插件树',
      nodes: buildPluginTree(tree),
    },
    {
      kind: 'graph',
      title: '插件关系图',
      nodes: [
        { id: 'registry', label: 'Registry', group: 'core' },
        { id: 'host', label: 'Extension Host', group: 'runtime' },
        { id: 'broker', label: 'Capability Broker', group: 'security' },
        ...plugins.slice(0, 5).map((plugin) => ({
          id: plugin.id,
          label: plugin.displayName,
          group: plugin.kind,
          value: plugin.id,
        })),
      ],
      edges: [
        { from: 'registry', label: 'loads', to: 'host' },
        { from: 'host', label: 'requests', to: 'broker' },
        ...plugins.slice(0, 5).map((plugin) => ({
          from: 'registry',
          label: plugin.parentPluginId ? 'mounts child' : 'registers',
          to: plugin.id,
        })),
      ],
    },
    {
      kind: 'timeline',
      title: '事件轨迹',
      items: eventBusRecords.value.slice(-8).map((record) => ({
        label: record.type,
        time: formatTime(record.timestamp),
        tone: eventTone(record.kind),
        value: `${record.kind} · ${record.source}${record.target ? ` -> ${record.target}` : ''} · ${formatPayload(record.payload)}`,
      })),
    },
    {
      kind: 'markdown',
      title: '平台说明',
      content: [
        '# AIO Plugin Platform',
        '',
        '- 技术栈：TypeScript + Rust',
        '- 核心：Permission Core / Capability Broker / Event Bus / Extension Host',
        '- 策略链：permission.policy 插件可声明 deny/warn 规则，运行时在基础校验后收紧权限',
        '- UI contract：summary-list / detail / form / table / tree / timeline / markdown',
        '- 发布闭环：publish gate / signature placeholder / lock / audit / rollback',
        '- 这页直接消费 Rust 注册表快照，作为插件化骨架的前端视图。',
      ].join('\n'),
    },
    {
      kind: 'wizard',
      title: '插件自举向导',
      activeStep: 'validate',
      steps: [
        {
          id: 'formula',
          title: '公式',
          description: '维护 formula.json 作为事实来源',
        },
        {
          id: 'generate',
          title: '生成',
          description: '生成胶囊源码和 smoke test',
        },
        { id: 'validate', title: '校验', description: '运行注册表和类型检查' },
        {
          id: 'publish',
          title: '发布',
          description: '写入本地 registry 并保留审计',
        },
      ],
    },
  ];
});
</script>

<template>
  <Page
    description="查看当前内置插件、系统胶囊和编译后的命令/权限注册表"
    title="插件注册表"
  >
    <div class="space-y-4">
      <ElAlert
        :closable="false"
        show-icon
        title="当前页面直接读取 Rust 插件注册表快照，并用同一份合同预览声明式视图。"
        type="info"
      />

      <section class="border-border/60 space-y-3 rounded-sm border p-3">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div>
            <div class="text-sm font-medium">App Runtime</div>
            <div class="text-muted-foreground text-xs">
              记录 start / stop / reload / workspace / session 生命周期，并把
              runtime 事件写入事件总线。
            </div>
          </div>
          <ElTag
            :type="runtimeSnapshot?.status === 'running' ? 'success' : 'info'"
            size="small"
          >
            {{ runtimeSnapshot?.status || '-' }}
          </ElTag>
        </div>
        <div class="flex flex-wrap gap-3 text-sm">
          <span>平台: {{ runtimeSnapshot?.platformId || '-' }}</span>
          <span>模式: {{ runtimeSnapshot?.mode || '-' }}</span>
          <span>工作区: {{ runtimeSnapshot?.workspace || '-' }}</span>
          <span>会话: {{ runtimeSnapshot?.sessionId || '-' }}</span>
          <span>重载: {{ runtimeSnapshot?.reloadCount ?? 0 }}</span>
          <span>更新时间: {{ runtimeUpdatedAtText }}</span>
        </div>
        <div class="grid gap-3 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto]">
          <ElInput
            v-model="runtimeStartForm.workspace"
            clearable
            placeholder="启动工作区，例如 /Users/zjarlin/IdeaProjects/aio"
          />
          <ElInput
            v-model="runtimeStartForm.sessionId"
            clearable
            placeholder="启动会话，可留空"
          />
          <ElButton
            :loading="runtimeActionLoading === runtimeActionKey('start')"
            type="primary"
            @click="startRuntime"
          >
            启动
          </ElButton>
          <ElInput
            v-model="runtimeReloadForm.workspace"
            clearable
            placeholder="重载时切换工作区，可留空"
          />
          <ElInput
            v-model="runtimeReloadForm.sessionId"
            clearable
            placeholder="重载时切换会话，可留空"
          />
          <ElButton
            :loading="runtimeActionLoading === runtimeActionKey('reload')"
            type="primary"
            @click="reloadRuntime"
          >
            重载
          </ElButton>
          <ElInput
            v-model="runtimeWorkspaceForm.workspace"
            clearable
            placeholder="设置 runtime 工作区"
          />
          <ElInput
            v-model="runtimeSessionForm.sessionId"
            clearable
            placeholder="设置 runtime 会话"
          />
          <div class="flex items-center gap-2">
            <ElButton
              :loading="runtimeActionLoading === runtimeActionKey('workspace')"
              @click="updateRuntimeWorkspace"
            >
              工作区
            </ElButton>
            <ElButton
              :loading="runtimeActionLoading === runtimeActionKey('session')"
              @click="updateRuntimeSession"
            >
              会话
            </ElButton>
            <ElButton
              :loading="runtimeActionLoading === runtimeActionKey('stop')"
              type="warning"
              @click="stopRuntime"
            >
              停止
            </ElButton>
          </div>
        </div>
        <div
          class="grid gap-3 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1fr)]"
        >
          <ElInput
            v-model="runtimeStartForm.mode"
            clearable
            placeholder="启动模式，默认 desktop"
          />
          <ElInput
            v-model="runtimeStartForm.reason"
            clearable
            placeholder="启动原因"
          />
          <ElInput
            v-model="runtimeStopForm.reason"
            clearable
            placeholder="停止原因"
          />
        </div>
      </section>

      <section class="border-border/60 space-y-3 rounded-sm border p-3">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div>
            <div class="text-sm font-medium">草稿验证</div>
            <div class="text-muted-foreground text-xs">
              运行 Plugin Factory Verification Runner，生成 diagnostics.json 和
              verification.json。
            </div>
          </div>
          <ElTag
            v-if="verificationReport"
            :type="
              verificationReport.status === 'passed' ? 'success' : 'danger'
            "
            size="small"
          >
            {{ verificationReport.status }}
          </ElTag>
        </div>
        <div
          class="grid gap-3 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto_auto]"
        >
          <ElInput
            v-model="verifyDraftSourcePath"
            clearable
            placeholder="插件草稿目录，例如 /private/tmp/aio-plugin-verify.8bQhMX"
          />
          <ElInput
            v-model="verifyDraftOutputDir"
            clearable
            placeholder="输出目录，留空写入草稿目录"
          />
          <div class="flex items-center gap-2 text-sm">
            <span>写入报告</span>
            <ElSwitch v-model="verifyDraftWrite" />
          </div>
          <ElButton
            :disabled="!verifyDraftSourcePath.trim()"
            :loading="verifyDraftLoading"
            type="primary"
            @click="verifyDraft"
          >
            验证草稿
          </ElButton>
        </div>
        <div v-if="verificationReport" class="space-y-3">
          <div class="flex flex-wrap gap-3 text-sm">
            <span>插件: {{ verificationReport.pluginId }}</span>
            <span>Run: {{ verificationReport.runId }}</span>
            <span>
              写入文件: {{ countItems(verificationReport.writtenFiles) }}
            </span>
          </div>
          <ElTable
            :data="verificationReport.checks || []"
            border
            size="small"
            stripe
          >
            <ElTableColumn label="检查" min-width="180" prop="title" />
            <ElTableColumn label="状态" width="100">
              <template #default="{ row }">
                <ElTag
                  :type="
                    row.status === 'passed'
                      ? 'success'
                      : row.status === 'warning'
                        ? 'warning'
                        : 'danger'
                  "
                  size="small"
                >
                  {{ row.status }}
                </ElTag>
              </template>
            </ElTableColumn>
            <ElTableColumn label="详情" min-width="260" prop="detail" />
          </ElTable>
          <div class="text-muted-foreground space-y-1 text-xs">
            <div>
              Repair Hint:
              {{ verificationReport.diagnostics?.repairHint || '-' }}
            </div>
            <div
              v-for="file in verificationReport.writtenFiles || []"
              :key="file"
            >
              {{ file }}
            </div>
          </div>
        </div>
      </section>

      <section class="border-border/60 space-y-3 rounded-sm border p-3">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div>
            <div class="text-sm font-medium">发布门禁</div>
            <div class="text-muted-foreground text-xs">
              检查 verification、lock、signature、audit 和远端 registry
              协议草案。
            </div>
          </div>
          <ElTag
            v-if="publishGateReport"
            :type="publishGateReport.status === 'passed' ? 'success' : 'danger'"
            size="small"
          >
            {{ publishGateReport.status }}
          </ElTag>
        </div>
        <div class="grid gap-3 xl:grid-cols-[minmax(0,1fr)_auto_auto]">
          <ElInput
            v-model="publishGateSourcePath"
            clearable
            placeholder="插件草稿目录，例如 /private/tmp/aio-plugin-publish"
          />
          <div class="flex items-center gap-2 text-sm">
            <span>写入门禁文件</span>
            <ElSwitch v-model="publishGateWrite" />
          </div>
          <ElButton
            :disabled="!publishGateSourcePath.trim()"
            :loading="publishGateLoading"
            type="primary"
            @click="runPublishGate"
          >
            运行门禁
          </ElButton>
        </div>
        <div v-if="publishGateReport" class="space-y-3">
          <div class="flex flex-wrap gap-3 text-sm">
            <span>插件: {{ publishGateReport.pluginId }}</span>
            <span>哈希: {{ publishGateReport.contentHash }}</span>
            <span>签名: {{ publishGateReport.signature.status }}</span>
          </div>
          <ElTable
            :data="publishGateReport.checks || []"
            border
            size="small"
            stripe
          >
            <ElTableColumn label="检查" min-width="180" prop="title" />
            <ElTableColumn label="状态" width="100">
              <template #default="{ row }">
                <ElTag
                  :type="
                    row.status === 'passed'
                      ? 'success'
                      : row.status === 'warning'
                        ? 'warning'
                        : 'danger'
                  "
                  size="small"
                >
                  {{ row.status }}
                </ElTag>
              </template>
            </ElTableColumn>
            <ElTableColumn label="详情" min-width="260" prop="detail" />
          </ElTable>
          <div class="text-muted-foreground space-y-1 text-xs">
            <div>
              远端协议: {{ publishGateReport.remoteRegistryProtocolPath }}
            </div>
            <div
              v-for="file in publishGateReport.writtenFiles || []"
              :key="file"
            >
              {{ file }}
            </div>
          </div>
        </div>
      </section>

      <div class="flex items-center gap-3 text-sm">
        <span>Runtime: {{ runtimeSnapshot?.status || '-' }}</span>
        <span>Schema: {{ snapshot?.schemaVersion || '-' }}</span>
        <span>插件数: {{ countItems(snapshot?.plugins) }}</span>
        <span>系统胶囊: {{ countItems(snapshot?.systemCapsules) }}</span>
        <span>能力: {{ countItems(snapshot?.capabilities) }}</span>
        <span>Provider: {{ countItems(snapshot?.capabilityProviders) }}</span>
        <span>工具: {{ countItems(snapshot?.tools) }}</span>
        <span>设置: {{ countItems(snapshot?.settings) }}</span>
        <span>路由: {{ countItems(snapshot?.routes) }}</span>
        <span>资源: {{ countItems(snapshot?.resources) }}</span>
        <span>策略: {{ countItems(snapshot?.policies) }}</span>
        <span>扩展点: {{ countItems(snapshot?.extensionPoints) }}</span>
        <span>事件声明: {{ countItems(snapshot?.events) }}</span>
        <span>树节点: {{ countItems(snapshot?.extensionTree?.nodes) }}</span>
        <span>权限审批: {{ countItems(permissionApprovalRecords) }}</span>
        <span>权限同意: {{ countItems(permissionConsentRecords) }}</span>
        <span>
          能力批准: {{ countItems(localState?.childCapabilityApprovals) }}
        </span>
        <span>诊断数: {{ countItems(snapshot?.diagnostics) }}</span>
        <ElButton :loading="loading" size="small" @click="reloadRegistry">
          重新加载
        </ElButton>
      </div>

      <section class="space-y-2">
        <div class="text-sm font-medium">插件</div>
        <ElTable
          :data="snapshot?.plugins || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="ID" min-width="180" prop="id" />
          <ElTableColumn label="类型" prop="kind" width="120" />
          <ElTableColumn label="名称" min-width="160" prop="displayName" />
          <ElTableColumn label="父级" min-width="180">
            <template #default="{ row }">
              <span>{{ row.parentPluginId || '-' }}</span>
            </template>
          </ElTableColumn>
          <ElTableColumn label="支持" min-width="140">
            <template #default="{ row }">
              <span>{{ row.platformSupported?.join(', ') || '-' }}</span>
            </template>
          </ElTableColumn>
          <ElTableColumn label="降级" min-width="140">
            <template #default="{ row }">
              <span>{{ row.platformDegraded?.join(', ') || '-' }}</span>
            </template>
          </ElTableColumn>
          <ElTableColumn label="不支持" min-width="140">
            <template #default="{ row }">
              <span>{{ row.platformUnsupported?.join(', ') || '-' }}</span>
            </template>
          </ElTableColumn>
          <ElTableColumn label="命令" width="80">
            <template #default="{ row }">
              <ElTag size="small">{{ countItems(row.commands) }}</ElTag>
            </template>
          </ElTableColumn>
          <ElTableColumn label="菜单" width="80">
            <template #default="{ row }">
              <ElTag size="small">{{ countItems(row.menus) }}</ElTag>
            </template>
          </ElTableColumn>
          <ElTableColumn label="视图" width="80">
            <template #default="{ row }">
              <ElTag size="small">{{ countItems(row.views) }}</ElTag>
            </template>
          </ElTableColumn>
          <ElTableColumn label="设置" width="80">
            <template #default="{ row }">
              <ElTag size="small">{{ countItems(row.settings) }}</ElTag>
            </template>
          </ElTableColumn>
          <ElTableColumn label="路由" width="80">
            <template #default="{ row }">
              <ElTag size="small">{{ countItems(row.routes) }}</ElTag>
            </template>
          </ElTableColumn>
          <ElTableColumn label="资源" width="80">
            <template #default="{ row }">
              <ElTag size="small">{{ countItems(row.resources) }}</ElTag>
            </template>
          </ElTableColumn>
          <ElTableColumn label="能力" width="80">
            <template #default="{ row }">
              <ElTag size="small">{{ countItems(row.capabilities) }}</ElTag>
            </template>
          </ElTableColumn>
          <ElTableColumn label="Provider" width="90">
            <template #default="{ row }">
              <ElTag size="small">
                {{ countItems(row.capabilityProviders) }}
              </ElTag>
            </template>
          </ElTableColumn>
          <ElTableColumn label="扩展点" width="90">
            <template #default="{ row }">
              <ElTag size="small">{{ countItems(row.extensionPoints) }}</ElTag>
            </template>
          </ElTableColumn>
        </ElTable>
      </section>

      <section class="space-y-2">
        <div class="text-sm font-medium">插件树</div>
        <ElTable
          :data="snapshot?.extensionTree?.nodes || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="插件" min-width="180" prop="pluginId" />
          <ElTableColumn label="父级" min-width="160" prop="parentPluginId" />
          <ElTableColumn label="挂载点" min-width="200" prop="parentMount" />
          <ElTableColumn label="深度" prop="depth" width="70" />
          <ElTableColumn label="子节点" prop="childCount" width="80" />
          <ElTableColumn label="贡献" min-width="180">
            <template #default="{ row }">
              <span>
                命令 {{ countItems(row.commands) }} / 视图
                {{ countItems(row.views) }} / 菜单
                {{ countItems(row.menus) }}
              </span>
            </template>
          </ElTableColumn>
          <ElTableColumn label="有效能力" min-width="180">
            <template #default="{ row }">
              <ElTag
                v-for="capability in row.effectiveCapabilities"
                :key="capability"
                class="mr-1"
                size="small"
              >
                {{ capability }}
              </ElTag>
              <span v-if="countItems(row.effectiveCapabilities) === 0">-</span>
            </template>
          </ElTableColumn>
          <ElTableColumn label="待审批能力" min-width="180">
            <template #default="{ row }">
              <div class="flex flex-wrap gap-2">
                <span
                  v-for="capability in row.capabilityEscalations"
                  :key="capability"
                  class="inline-flex items-center gap-1"
                >
                  <ElTag size="small" type="warning">
                    {{ capability }}
                  </ElTag>
                  <ElButton
                    :loading="
                      childCapabilityDecisionKey ===
                      childCapabilityKey(
                        row.parentPluginId || '',
                        row.pluginId,
                        capability,
                      )
                    "
                    link
                    size="small"
                    type="primary"
                    @click="approveChildCapability(row, capability)"
                  >
                    批准
                  </ElButton>
                </span>
              </div>
              <span v-if="countItems(row.capabilityEscalations) === 0">-</span>
            </template>
          </ElTableColumn>
        </ElTable>
      </section>

      <section class="space-y-2">
        <div class="text-sm font-medium">挂载贡献</div>
        <ElTable
          :data="snapshot?.extensionTree?.mounts || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="父插件" min-width="160" prop="parentPluginId" />
          <ElTableColumn
            label="扩展点"
            min-width="200"
            prop="extensionPointId"
          />
          <ElTableColumn label="子插件" min-width="180" prop="childPluginId" />
          <ElTableColumn
            label="兼容范围"
            prop="compatibleParentRange"
            width="120"
          />
          <ElTableColumn label="命令" width="80">
            <template #default="{ row }">
              <ElTag size="small">{{ countItems(row.commands) }}</ElTag>
            </template>
          </ElTableColumn>
          <ElTableColumn label="视图" width="80">
            <template #default="{ row }">
              <ElTag size="small">{{ countItems(row.views) }}</ElTag>
            </template>
          </ElTableColumn>
          <ElTableColumn label="菜单" width="80">
            <template #default="{ row }">
              <ElTag size="small">{{ countItems(row.menus) }}</ElTag>
            </template>
          </ElTableColumn>
          <ElTableColumn label="设置" width="80">
            <template #default="{ row }">
              <ElTag size="small">{{ countItems(row.settings) }}</ElTag>
            </template>
          </ElTableColumn>
          <ElTableColumn label="资源" width="80">
            <template #default="{ row }">
              <ElTag size="small">{{ countItems(row.resources) }}</ElTag>
            </template>
          </ElTableColumn>
        </ElTable>
      </section>

      <section class="space-y-2">
        <div class="text-sm font-medium">系统胶囊</div>
        <ElTable
          :data="snapshot?.systemCapsules || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="ID" min-width="220" prop="id" />
          <ElTableColumn label="名称" min-width="160" prop="displayName" />
          <ElTableColumn label="命令" width="80">
            <template #default="{ row }">
              <ElTag size="small">{{ countItems(row.commands) }}</ElTag>
            </template>
          </ElTableColumn>
          <ElTableColumn label="菜单" width="80">
            <template #default="{ row }">
              <ElTag size="small">{{ countItems(row.menus) }}</ElTag>
            </template>
          </ElTableColumn>
        </ElTable>
      </section>

      <section class="space-y-2">
        <div class="text-sm font-medium">命令</div>
        <ElTable
          :data="snapshot?.commands || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="ID" min-width="180" prop="id" />
          <ElTableColumn label="标题" min-width="180" prop="title" />
          <ElTableColumn label="来源" min-width="160" prop="sourceId" />
          <ElTableColumn label="类别" min-width="120" prop="category" />
          <ElTableColumn label="条件" min-width="220" prop="when" />
        </ElTable>
      </section>

      <section class="space-y-2">
        <div class="text-sm font-medium">工具</div>
        <ElTable
          :data="snapshot?.tools || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="ID" min-width="180" prop="id" />
          <ElTableColumn label="标题" min-width="180" prop="title" />
          <ElTableColumn label="来源" min-width="160" prop="sourceId" />
          <ElTableColumn label="类别" min-width="120" prop="category" />
          <ElTableColumn label="条件" min-width="220" prop="when" />
        </ElTable>
      </section>

      <section class="space-y-2">
        <div class="text-sm font-medium">设置</div>
        <ElTable
          :data="snapshot?.settings || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="ID" min-width="180" prop="id" />
          <ElTableColumn label="标题" min-width="180" prop="title" />
          <ElTableColumn label="Schema" min-width="180" prop="schema" />
          <ElTableColumn label="来源" min-width="160" prop="sourceId" />
          <ElTableColumn label="类型" min-width="120" prop="sourceKind" />
        </ElTable>
      </section>

      <section class="space-y-2">
        <div class="text-sm font-medium">路由</div>
        <ElTable
          :data="snapshot?.routes || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="ID" min-width="180" prop="id" />
          <ElTableColumn label="标题" min-width="160" prop="title" />
          <ElTableColumn label="路径" min-width="180" prop="path" />
          <ElTableColumn label="组件" min-width="220" prop="component" />
          <ElTableColumn label="来源" min-width="160" prop="sourceId" />
          <ElTableColumn label="条件" min-width="220" prop="when" />
        </ElTable>
      </section>

      <section class="space-y-2">
        <div class="text-sm font-medium">资源</div>
        <ElTable
          :data="snapshot?.resources || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="ID" min-width="180" prop="id" />
          <ElTableColumn label="标题" min-width="180" prop="title" />
          <ElTableColumn label="类型" min-width="120" prop="kind" />
          <ElTableColumn label="Schema" min-width="180" prop="schema" />
          <ElTableColumn label="来源" min-width="160" prop="sourceId" />
        </ElTable>
      </section>

      <section class="space-y-2">
        <div class="text-sm font-medium">视图</div>
        <ElTable
          :data="snapshot?.views || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="ID" min-width="180" prop="id" />
          <ElTableColumn label="Schema" min-width="140" prop="schema" />
          <ElTableColumn label="Slot" min-width="120" prop="slot" />
          <ElTableColumn label="路径" min-width="180" prop="path" />
          <ElTableColumn label="来源" min-width="160" prop="sourceId" />
          <ElTableColumn label="条件" min-width="220" prop="when" />
        </ElTable>
      </section>

      <section class="space-y-2">
        <div class="text-sm font-medium">权限种子</div>
        <ElTable
          :data="snapshot?.permissionSeeds || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="编码" min-width="160" prop="code" />
          <ElTableColumn label="名称" min-width="160" prop="name" />
          <ElTableColumn label="类型" prop="permissionType" width="90" />
          <ElTableColumn label="父级" min-width="140" prop="parentId" />
          <ElTableColumn label="普通用户" width="90">
            <template #default="{ row }">
              <ElTag :type="row.ordinaryUser ? 'success' : 'info'" size="small">
                {{ row.ordinaryUser ? '是' : '否' }}
              </ElTag>
            </template>
          </ElTableColumn>
        </ElTable>
      </section>

      <section class="space-y-2">
        <div class="text-sm font-medium">能力</div>
        <ElTable
          :data="snapshot?.capabilities || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="能力" min-width="160" prop="id" />
          <ElTableColumn label="作用域" min-width="180" prop="scope" />
          <ElTableColumn label="来源" min-width="160" prop="sourceId" />
          <ElTableColumn label="原因" min-width="260" prop="reason" />
          <ElTableColumn fixed="right" label="操作" width="90">
            <template #default="{ row }">
              <ElButton
                link
                type="primary"
                @click="fillConsentFromCapability(row)"
              >
                授权
              </ElButton>
            </template>
          </ElTableColumn>
        </ElTable>
      </section>

      <section class="space-y-2">
        <div class="text-sm font-medium">能力 Provider</div>
        <ElTable
          :data="snapshot?.capabilityProviders || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="Provider" min-width="220" prop="id" />
          <ElTableColumn label="能力" min-width="160" prop="capability" />
          <ElTableColumn label="类型" prop="kind" width="100" />
          <ElTableColumn label="信任" min-width="150">
            <template #default="{ row }">
              <ElTag
                :type="
                  row.trustLevel === 'platform'
                    ? 'success'
                    : row.trustLevel === 'trusted-provider'
                      ? 'warning'
                      : 'info'
                "
                size="small"
              >
                {{ row.trustLevel }}
              </ElTag>
            </template>
          </ElTableColumn>
          <ElTableColumn label="平台" min-width="180">
            <template #default="{ row }">
              <span>{{ row.platforms?.join(', ') || '-' }}</span>
            </template>
          </ElTableColumn>
          <ElTableColumn label="来源" min-width="180" prop="sourceId" />
          <ElTableColumn label="入口" min-width="240" prop="entry" />
          <ElTableColumn label="降级" min-width="180" prop="fallback" />
        </ElTable>
      </section>

      <section class="space-y-2">
        <div class="text-sm font-medium">权限策略</div>
        <ElTable
          :data="snapshot?.policies || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="ID" min-width="200" prop="id" />
          <ElTableColumn label="效果" width="90">
            <template #default="{ row }">
              <ElTag
                :type="row.effect === 'deny' ? 'danger' : 'warning'"
                size="small"
              >
                {{ row.effect }}
              </ElTag>
            </template>
          </ElTableColumn>
          <ElTableColumn label="优先级" prop="priority" width="90" />
          <ElTableColumn label="来源" min-width="180" prop="sourceId" />
          <ElTableColumn label="匹配能力" min-width="180">
            <template #default="{ row }">
              <span>{{ row.capabilities?.join(', ') || '*' }}</span>
            </template>
          </ElTableColumn>
          <ElTableColumn label="匹配作用域" min-width="180">
            <template #default="{ row }">
              <span>{{ row.scopes?.join(', ') || '*' }}</span>
            </template>
          </ElTableColumn>
          <ElTableColumn label="目标包含" min-width="160">
            <template #default="{ row }">
              <span>{{ row.targetContains?.join(', ') || '-' }}</span>
            </template>
          </ElTableColumn>
          <ElTableColumn label="原因" min-width="260" prop="reason" />
        </ElTable>
      </section>

      <section class="border-border/60 space-y-3 rounded-sm border p-3">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div>
            <div class="text-sm font-medium">权限同意</div>
            <div class="text-muted-foreground text-xs">
              当前登录用户对插件来源、能力和作用域的持久 consent，Permission
              Core 运行时按此结果放行。
            </div>
          </div>
          <ElTag size="small">{{ countItems(permissionConsentRecords) }}</ElTag>
        </div>
        <div
          class="grid gap-3 xl:grid-cols-[minmax(0,1fr)_160px_minmax(0,1fr)_minmax(0,1fr)_minmax(0,1fr)_auto]"
        >
          <ElInput
            v-model="consentGrantForm.sourceId"
            clearable
            placeholder="sourceId，例如 platform.capability-broker"
          />
          <ElSelect v-model="consentGrantForm.sourceKind" class="w-full">
            <ElOption label="plugin" value="plugin" />
            <ElOption label="child-plugin" value="child-plugin" />
            <ElOption label="system" value="system" />
          </ElSelect>
          <ElInput
            v-model="consentGrantForm.capability"
            clearable
            placeholder="capability，例如 fs.read"
          />
          <ElInput
            v-model="consentGrantForm.scope"
            clearable
            placeholder="scope，留空等同 *"
          />
          <ElInput
            v-model="consentGrantForm.reason"
            clearable
            placeholder="授权原因"
          />
          <ElButton
            :disabled="
              !consentGrantForm.sourceId.trim() ||
              !consentGrantForm.capability.trim()
            "
            :loading="consentGrantLoading"
            type="primary"
            @click="grantConsent"
          >
            授权
          </ElButton>
        </div>
        <ElTable
          :data="permissionConsentRecords"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="状态" width="90">
            <template #default="{ row }">
              <ElTag
                :type="row.status === 'granted' ? 'success' : 'info'"
                size="small"
              >
                {{ row.status === 'granted' ? '已授权' : '已撤销' }}
              </ElTag>
            </template>
          </ElTableColumn>
          <ElTableColumn label="来源" min-width="200" prop="sourceId" />
          <ElTableColumn label="类型" prop="sourceKind" width="120" />
          <ElTableColumn label="能力" min-width="160" prop="capability" />
          <ElTableColumn label="作用域" min-width="180" prop="scope" />
          <ElTableColumn label="原因" min-width="240" prop="reason" />
          <ElTableColumn label="更新时间" width="170">
            <template #default="{ row }">
              <span>{{ formatTime(row.updatedAt) }}</span>
            </template>
          </ElTableColumn>
          <ElTableColumn fixed="right" label="操作" width="100">
            <template #default="{ row }">
              <ElButton
                :disabled="row.status !== 'granted'"
                :loading="consentRevokeKey === consentRowKey(row)"
                link
                type="danger"
                @click="revokeConsent(row)"
              >
                撤销
              </ElButton>
            </template>
          </ElTableColumn>
        </ElTable>
      </section>

      <section class="border-border/60 space-y-3 rounded-sm border p-3">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div>
            <div class="text-sm font-medium">权限审批</div>
            <div class="text-muted-foreground text-xs">
              Capability Broker 缺少运行时 consent 时自动创建 pending
              请求；通过后会写入同意记录，后续同作用域调用直接放行。
            </div>
          </div>
          <ElTag size="small">
            {{ countItems(permissionApprovalRecords) }}
          </ElTag>
        </div>
        <ElTable
          :data="permissionApprovalRecords"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="状态" width="90">
            <template #default="{ row }">
              <ElTag
                :type="
                  row.status === 'pending'
                    ? 'warning'
                    : row.status === 'approved'
                      ? 'success'
                      : 'danger'
                "
                size="small"
              >
                {{
                  row.status === 'pending'
                    ? '待审批'
                    : row.status === 'approved'
                      ? '已通过'
                      : '已拒绝'
                }}
              </ElTag>
            </template>
          </ElTableColumn>
          <ElTableColumn label="来源" min-width="200" prop="sourceId" />
          <ElTableColumn label="类型" prop="sourceKind" width="120" />
          <ElTableColumn label="能力" min-width="160" prop="capability" />
          <ElTableColumn label="作用域" min-width="180" prop="scope" />
          <ElTableColumn label="目标" min-width="220" prop="target" />
          <ElTableColumn label="请求原因" min-width="240" prop="reason" />
          <ElTableColumn
            label="审批原因"
            min-width="220"
            prop="decisionReason"
          />
          <ElTableColumn label="更新时间" width="170">
            <template #default="{ row }">
              <span>{{ formatTime(row.updatedAt) }}</span>
            </template>
          </ElTableColumn>
          <ElTableColumn fixed="right" label="操作" width="130">
            <template #default="{ row }">
              <ElButton
                :disabled="row.status !== 'pending'"
                :loading="approvalDecisionKey === approvalRowKey(row)"
                link
                type="primary"
                @click="approveApproval(row)"
              >
                通过
              </ElButton>
              <ElButton
                :disabled="row.status !== 'pending'"
                :loading="approvalDecisionKey === approvalRowKey(row)"
                link
                type="danger"
                @click="denyApproval(row)"
              >
                拒绝
              </ElButton>
            </template>
          </ElTableColumn>
        </ElTable>
      </section>

      <section class="space-y-2">
        <div class="text-sm font-medium">扩展点</div>
        <ElTable
          :data="snapshot?.extensionPoints || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="ID" min-width="200" prop="id" />
          <ElTableColumn label="标题" min-width="160" prop="title" />
          <ElTableColumn label="来源" min-width="160" prop="sourceId" />
          <ElTableColumn label="合同" min-width="220" prop="contract" />
        </ElTable>
      </section>

      <section class="space-y-2">
        <div class="text-sm font-medium">声明事件</div>
        <ElTable
          :data="snapshot?.events || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="事件" min-width="220" prop="event" />
          <ElTableColumn label="Schema" min-width="220" prop="schema" />
          <ElTableColumn label="方向" prop="direction" width="110" />
          <ElTableColumn label="来源" min-width="180" prop="sourceId" />
          <ElTableColumn label="类型" prop="sourceKind" width="120" />
        </ElTable>
      </section>

      <section class="space-y-2">
        <div class="text-sm font-medium">诊断</div>
        <ElTable
          :data="snapshot?.diagnostics || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="消息">
            <template #default="{ row }">
              <span>{{ row }}</span>
            </template>
          </ElTableColumn>
        </ElTable>
      </section>

      <section class="space-y-2">
        <div class="text-sm font-medium">事件总线</div>
        <ElTable
          :data="eventBusRecords"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="类型" min-width="180" prop="type" />
          <ElTableColumn label="Schema" min-width="220" prop="schema" />
          <ElTableColumn label="消息" prop="kind" width="100" />
          <ElTableColumn label="流" width="150">
            <template #default="{ row }">
              <span>{{ row.correlationId || '-' }}</span>
            </template>
          </ElTableColumn>
          <ElTableColumn label="序号" width="90">
            <template #default="{ row }">
              <span>{{ row.sequence ?? '-' }}</span>
            </template>
          </ElTableColumn>
          <ElTableColumn label="结束" width="80">
            <template #default="{ row }">
              <ElTag
                v-if="row.kind === 'stream'"
                :type="row.done ? 'success' : 'info'"
                size="small"
              >
                {{ row.done ? '是' : '否' }}
              </ElTag>
              <span v-else>-</span>
            </template>
          </ElTableColumn>
          <ElTableColumn label="来源" min-width="160" prop="source" />
          <ElTableColumn label="目标" min-width="140">
            <template #default="{ row }">
              <span>{{ row.target || '-' }}</span>
            </template>
          </ElTableColumn>
          <ElTableColumn label="时间" width="170">
            <template #default="{ row }">
              <span>{{ formatTime(row.timestamp) }}</span>
            </template>
          </ElTableColumn>
          <ElTableColumn label="Payload" min-width="260">
            <template #default="{ row }">
              <span>{{ formatPayload(row.payload) }}</span>
            </template>
          </ElTableColumn>
        </ElTable>
      </section>

      <section class="space-y-2">
        <div class="text-sm font-medium">权限审计</div>
        <ElTable
          :data="permissionAuditRecords"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="决策" prop="decision" width="90" />
          <ElTableColumn label="能力" min-width="160" prop="capability" />
          <ElTableColumn label="来源" min-width="180" prop="sourceId" />
          <ElTableColumn label="来源类型" prop="sourceKind" width="120" />
          <ElTableColumn label="作用域" min-width="180" prop="scope" />
          <ElTableColumn label="目标" min-width="180" prop="target" />
          <ElTableColumn label="原因" min-width="260" prop="reason" />
          <ElTableColumn fixed="right" label="操作" width="90">
            <template #default="{ row }">
              <ElButton
                link
                type="primary"
                @click="fillConsentFromDecision(row)"
              >
                授权
              </ElButton>
            </template>
          </ElTableColumn>
        </ElTable>
      </section>

      <section class="space-y-3">
        <div class="text-sm font-medium">视图合同预览</div>
        <div class="grid gap-4 xl:grid-cols-2">
          <PluginViewRenderer
            v-for="view in viewPreviews"
            :key="view.kind + (view.title || '')"
            :view="view"
          />
        </div>
      </section>

      <section class="space-y-2">
        <div class="text-sm font-medium">本地 registry</div>
        <div class="flex items-center gap-3 text-sm">
          <span>安装: {{ countItems(localState?.installed) }}</span>
          <span>锁定: {{ countItems(localState?.locks) }}</span>
          <span>
            能力批准: {{ countItems(localState?.childCapabilityApprovals) }}
          </span>
          <span>历史: {{ countItems(localState?.history) }}</span>
          <span>审计: {{ countItems(localState?.audits) }}</span>
        </div>
        <div class="grid gap-3 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto]">
          <ElInput v-model="rollbackId" clearable placeholder="回滚插件 ID" />
          <ElInput
            v-model="rollbackContentHash"
            clearable
            placeholder="可选：回滚到指定 contentHash，留空回滚到上一版"
          />
          <ElButton
            :disabled="!rollbackId.trim()"
            :loading="rollbackLoading"
            type="warning"
            @click="rollbackPlugin()"
          >
            回滚
          </ElButton>
        </div>
        <div v-if="rollbackResult" class="text-muted-foreground text-xs">
          已回滚 {{ rollbackResult.restored.id }}：
          {{ rollbackResult.previousContentHash }} ->
          {{ rollbackResult.restored.contentHash }}
        </div>
        <ElTable
          :data="localState?.installed || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="ID" min-width="180" prop="id" />
          <ElTableColumn label="类型" prop="kind" width="120" />
          <ElTableColumn label="启用" width="80">
            <template #default="{ row }">
              <ElTag :type="row.enabled ? 'success' : 'info'" size="small">
                {{ row.enabled ? '是' : '否' }}
              </ElTag>
            </template>
          </ElTableColumn>
          <ElTableColumn label="来源" min-width="220" prop="sourcePath" />
          <ElTableColumn label="哈希" min-width="180" prop="contentHash" />
          <ElTableColumn label="版本" prop="version" width="100" />
          <ElTableColumn label="操作" width="100">
            <template #default="{ row }">
              <ElButton
                :loading="rollbackLoading && rollbackId === row.id"
                link
                type="warning"
                @click="rollbackPlugin(row.id)"
              >
                上一版
              </ElButton>
            </template>
          </ElTableColumn>
        </ElTable>

        <ElTable
          :data="localState?.childCapabilityApprovals || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="状态" width="90">
            <template #default="{ row }">
              <ElTag
                :type="row.status === 'approved' ? 'success' : 'info'"
                size="small"
              >
                {{ row.status === 'approved' ? '已批准' : '已撤销' }}
              </ElTag>
            </template>
          </ElTableColumn>
          <ElTableColumn label="父插件" min-width="180" prop="parentPluginId" />
          <ElTableColumn label="子插件" min-width="180" prop="childPluginId" />
          <ElTableColumn label="能力" min-width="160" prop="capability" />
          <ElTableColumn label="批准原因" min-width="240" prop="reason" />
          <ElTableColumn
            label="撤销原因"
            min-width="220"
            prop="revokedReason"
          />
          <ElTableColumn label="更新时间" width="170">
            <template #default="{ row }">
              <span>{{ formatTime(row.updatedAt) }}</span>
            </template>
          </ElTableColumn>
          <ElTableColumn label="操作" width="100">
            <template #default="{ row }">
              <ElButton
                :disabled="row.status !== 'approved'"
                :loading="
                  childCapabilityDecisionKey === childCapabilityApprovalKey(row)
                "
                link
                type="danger"
                @click="revokeChildCapability(row)"
              >
                撤销
              </ElButton>
            </template>
          </ElTableColumn>
        </ElTable>

        <ElTable
          :data="localState?.history || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="历史插件" min-width="180" prop="id" />
          <ElTableColumn label="版本" prop="version" width="100" />
          <ElTableColumn label="哈希" min-width="180" prop="contentHash" />
          <ElTableColumn label="签名" min-width="220" prop="signaturePath" />
          <ElTableColumn label="历史路径" min-width="240" prop="registryPath" />
          <ElTableColumn label="操作" width="100">
            <template #default="{ row }">
              <ElButton
                :loading="rollbackLoading && rollbackId === row.id"
                link
                type="warning"
                @click="rollbackPlugin(row.id, row.contentHash)"
              >
                回滚
              </ElButton>
            </template>
          </ElTableColumn>
        </ElTable>

        <ElTable
          :data="localState?.audits || []"
          border
          size="small"
          stripe
          v-loading="loading"
        >
          <ElTableColumn label="动作" prop="action" width="120" />
          <ElTableColumn label="ID" min-width="180" prop="id" />
          <ElTableColumn label="状态" prop="status" width="90" />
          <ElTableColumn label="路径" min-width="220" prop="path" />
        </ElTable>
      </section>
    </div>
  </Page>
</template>
