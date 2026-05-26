import type {
  AppRuntimeLifecycleRecord,
  AppRuntimeReloadInput,
  AppRuntimeSessionInput,
  AppRuntimeSnapshot,
  AppRuntimeStartInput,
  AppRuntimeStopInput,
  AppRuntimeWorkspaceInput,
  BrowserOpenUrlInput,
  CapabilityAuditRecord,
  CapabilityInvokeInput,
  ChildCapabilityApprovalInput,
  ChildCapabilityApprovalRecord,
  EventBusPublishInput,
  EventBusSnapshotRequest,
  EventBusStreamInput,
  ExtensionHostPluginRecord,
  ExtensionHostSourceInput,
  FsReadInput,
  FsReadResult,
  FsWriteInput,
  FsWriteResult,
  PermissionApprovalDecisionInput,
  PermissionApprovalRecord,
  PermissionConsentGrantInput,
  PermissionConsentRecord,
  PermissionConsentRevokeInput,
  PermissionDecisionRecord,
  PlatformEventRecord,
  PluginCreateFromPromptInput,
  PluginDraftCreationResult,
  PluginPublishGateReport,
  PluginRegistryInstalledRecord,
  PluginRegistryLocalState,
  PluginRegistryRollbackInput,
  PluginRegistryRollbackResult,
  PluginRegistrySnapshot,
  PluginRepairFromDiagnosticsInput,
  PluginVerificationReport,
  PluginVerifyDraftInput,
  ProcessExecInput,
  ProcessExecResult,
} from '@vben/types';

import { callAuthedCommand, type PageRequest, type PageResult } from './client';

export interface UserRecord {
  avatar: string;
  createdAt: number;
  homePath: string;
  id: string;
  realName: string;
  roles: string;
  status: string;
  updatedAt: number;
  username: string;
}

export interface RoleRecord {
  code: string;
  createdAt: number;
  description: string;
  id: string;
  name: string;
  status: string;
  updatedAt: number;
}

export interface PermissionRecord {
  code: string;
  component: string;
  icon: string;
  id: string;
  name: string;
  parentId?: null | string;
  path: string;
  permissionType: 'button' | 'menu';
  sortOrder: number;
  status: string;
}

export interface DictTypeRecord {
  code: string;
  createdAt: number;
  description: string;
  id: string;
  name: string;
  sortOrder: number;
  status: string;
  updatedAt: number;
}

export interface DictItemRecord {
  createdAt: number;
  id: string;
  label: string;
  sortOrder: number;
  status: string;
  typeId: string;
  updatedAt: number;
  value: string;
}

export interface NoteRecord {
  category: string;
  content: string;
  createdAt: number;
  id: string;
  isArchived: boolean;
  isFavorite: boolean;
  ownerId: string;
  tags: string[];
  title: string;
  updatedAt: number;
}

export interface SkillRecord {
  category: string;
  code: string;
  createdAt: number;
  description: string;
  id: string;
  name: string;
  prompt: string;
  sortOrder: number;
  status: string;
  tags: string[];
  updatedAt: number;
}

export type AgentPreferenceSection =
  | 'design_patterns'
  | 'domain'
  | 'personal'
  | 'public';

export interface AgentPreferenceRecord {
  code: string;
  content: string;
  createdAt: number;
  domain: string;
  id: string;
  rationale: string;
  section: AgentPreferenceSection;
  sortOrder: number;
  status: string;
  tags: string[];
  title: string;
  updatedAt: number;
}

export interface AgentPreferencePageRequest extends PageRequest {
  domain?: string;
  section?: AgentPreferenceSection;
  status?: string;
}

export interface AgentPreferenceInput {
  code: string;
  content?: string;
  domain?: string;
  rationale?: string;
  section: AgentPreferenceSection;
  sortOrder?: number;
  status?: string;
  tags?: string[];
  title: string;
}

export interface AgentPreferenceUpdateInput extends AgentPreferenceInput {
  id: string;
}

export type AssetItemKind =
  | 'bash_functions'
  | 'cli'
  | 'docker_compose'
  | 'dotfiles'
  | 'env_vars';

export interface AssetValidationIssue {
  message: string;
  path: string;
  severity: 'error' | 'warning';
}

export interface AssetVariableCandidate {
  key: string;
  kind: string;
  occurrences: number;
  scope: string;
  source: string;
  value: string;
}

export interface AssetItemRecord {
  category: string;
  code: string;
  content: string;
  contentHash: string;
  createdAt: number;
  description: string;
  fileName: string;
  id: string;
  images: string[];
  kind: AssetItemKind;
  lastSyncedAt: number;
  name: string;
  ports: string[];
  serviceCount: number;
  services: string[];
  sortOrder: number;
  status: string;
  sourceMtime: number;
  sourcePath: string;
  sourceSize: number;
  tags: string[];
  updatedAt: number;
  validationIssues: AssetValidationIssue[];
  validationStatus: 'error' | 'unknown' | 'valid' | 'warning';
  variableCandidates: AssetVariableCandidate[];
  volumes: string[];
}

export interface AssetVariableRecord {
  assetItemId?: null | string;
  category: string;
  createdAt: number;
  defaultValue: string;
  description: string;
  id: string;
  key: string;
  kind: AssetItemKind;
  scope: 'file' | 'grid';
  sortOrder: number;
  source: string;
  status: string;
  updatedAt: number;
  value: string;
  valueKind: string;
}

export interface AssetItemPageRequest extends PageRequest {
  categories?: string[];
  category?: string;
  kind: AssetItemKind;
  status?: string;
}

export interface AssetItemInput {
  category?: string;
  code: string;
  content?: string;
  contentHash?: string;
  description?: string;
  fileName?: string;
  images?: string[];
  kind: AssetItemKind;
  lastSyncedAt?: number;
  name: string;
  ports?: string[];
  serviceCount?: number;
  services?: string[];
  sortOrder?: number;
  status?: string;
  sourceMtime?: number;
  sourcePath?: string;
  sourceSize?: number;
  tags?: string[];
  validationIssues?: AssetValidationIssue[];
  validationStatus?: 'error' | 'unknown' | 'valid' | 'warning';
  variableCandidates?: AssetVariableCandidate[];
  volumes?: string[];
}

export interface AssetItemUpdateInput extends AssetItemInput {
  id: string;
}

export interface AssetVariablePageRequest extends PageRequest {
  assetItemId?: string;
  category?: string;
  kind: AssetItemKind;
  scope?: 'file' | 'grid';
  status?: string;
}

export interface AssetVariableInput {
  assetItemId?: string;
  category?: string;
  defaultValue?: string;
  description?: string;
  id?: string;
  key: string;
  kind: AssetItemKind;
  sortOrder?: number;
  source?: string;
  status?: string;
  value?: string;
  valueKind?: string;
}

export async function userPageApi(request: PageRequest) {
  return await callAuthedCommand<PageResult<UserRecord>>('user_page', {
    request,
  });
}

export async function userCreateApi(input: Record<string, unknown>) {
  return await callAuthedCommand<UserRecord>('user_create', { input });
}

export async function userUpdateApi(input: Record<string, unknown>) {
  return await callAuthedCommand<UserRecord>('user_update', { input });
}

export async function userDisableApi(id: string) {
  return await callAuthedCommand<UserRecord>('user_disable', { id });
}

export async function userResetPasswordApi(input: Record<string, unknown>) {
  return await callAuthedCommand<null>('user_reset_password', { input });
}

export async function userDeleteApi(id: string) {
  return await callAuthedCommand<null>('user_delete', { id });
}

export async function rolePageApi(request: PageRequest) {
  return await callAuthedCommand<PageResult<RoleRecord>>('role_page', {
    request,
  });
}

export async function roleCreateApi(input: Record<string, unknown>) {
  return await callAuthedCommand<RoleRecord>('role_create', { input });
}

export async function roleUpdateApi(input: Record<string, unknown>) {
  return await callAuthedCommand<RoleRecord>('role_update', { input });
}

export async function roleDeleteApi(id: string) {
  return await callAuthedCommand<null>('role_delete', { id });
}

export async function roleAssignPermissionsApi(input: Record<string, unknown>) {
  return await callAuthedCommand<null>('role_assign_permissions', { input });
}

export async function rolePermissionIdsApi(roleId: string) {
  return await callAuthedCommand<string[]>('role_permission_ids', { roleId });
}

export async function permissionTreeApi() {
  return await callAuthedCommand<PermissionRecord[]>('permission_tree');
}

export async function permissionSaveApi(input: Record<string, unknown>) {
  return await callAuthedCommand<PermissionRecord>('permission_save', {
    input,
  });
}

export async function dictTypePageApi(request: PageRequest) {
  return await callAuthedCommand<PageResult<DictTypeRecord>>('dict_type_page', {
    request,
  });
}

export async function dictTypeCreateApi(input: Record<string, unknown>) {
  return await callAuthedCommand<DictTypeRecord>('dict_type_create', { input });
}

export async function dictTypeUpdateApi(input: Record<string, unknown>) {
  return await callAuthedCommand<DictTypeRecord>('dict_type_update', { input });
}

export async function dictTypeDeleteApi(id: string) {
  return await callAuthedCommand<null>('dict_type_delete', { id });
}

export async function dictItemPageApi(
  request: { typeId?: string } & PageRequest,
) {
  return await callAuthedCommand<PageResult<DictItemRecord>>('dict_item_page', {
    request,
  });
}

export async function dictItemCreateApi(input: Record<string, unknown>) {
  return await callAuthedCommand<DictItemRecord>('dict_item_create', { input });
}

export async function dictItemUpdateApi(input: Record<string, unknown>) {
  return await callAuthedCommand<DictItemRecord>('dict_item_update', { input });
}

export async function dictItemDeleteApi(id: string) {
  return await callAuthedCommand<null>('dict_item_delete', { id });
}

export async function notePageApi(
  request: { archived?: boolean; category?: string } & PageRequest,
) {
  return await callAuthedCommand<PageResult<NoteRecord>>('note_page', {
    request,
  });
}

export async function noteCreateApi(input: Record<string, unknown>) {
  return await callAuthedCommand<NoteRecord>('note_create', { input });
}

export async function noteUpdateApi(input: Record<string, unknown>) {
  return await callAuthedCommand<NoteRecord>('note_update', { input });
}

export async function noteDeleteApi(id: string) {
  return await callAuthedCommand<null>('note_delete', { id });
}

export async function noteArchiveApi(id: string, value: boolean) {
  return await callAuthedCommand<NoteRecord>('note_archive', {
    input: { id, value },
  });
}

export async function noteFavoriteApi(id: string, value: boolean) {
  return await callAuthedCommand<NoteRecord>('note_favorite', {
    input: { id, value },
  });
}

export async function skillPageApi(
  request: {
    categories?: string[];
    category?: string;
    status?: string;
  } & PageRequest,
) {
  return await callAuthedCommand<PageResult<SkillRecord>>('skill_page', {
    request,
  });
}

export async function skillCreateApi(input: Record<string, unknown>) {
  return await callAuthedCommand<SkillRecord>('skill_create', { input });
}

export async function skillUpdateApi(input: Record<string, unknown>) {
  return await callAuthedCommand<SkillRecord>('skill_update', { input });
}

export async function skillDeleteApi(id: string) {
  return await callAuthedCommand<null>('skill_delete', { id });
}

export async function skillToggleApi(id: string, status: string) {
  return await callAuthedCommand<SkillRecord>('skill_toggle', {
    input: { id, status },
  });
}

export async function agentPreferencePageApi(
  request: AgentPreferencePageRequest,
) {
  return await callAuthedCommand<PageResult<AgentPreferenceRecord>>(
    'agent_preference_page',
    { request },
  );
}

export async function agentPreferenceCreateApi(input: AgentPreferenceInput) {
  return await callAuthedCommand<AgentPreferenceRecord>(
    'agent_preference_create',
    { input },
  );
}

export async function agentPreferenceUpdateApi(
  input: AgentPreferenceUpdateInput,
) {
  return await callAuthedCommand<AgentPreferenceRecord>(
    'agent_preference_update',
    { input },
  );
}

export async function agentPreferenceDeleteApi(id: string) {
  return await callAuthedCommand<null>('agent_preference_delete', { id });
}

export async function agentPreferenceToggleApi(id: string, status: string) {
  return await callAuthedCommand<AgentPreferenceRecord>(
    'agent_preference_toggle',
    { input: { id, status } },
  );
}

export async function assetItemPageApi(request: AssetItemPageRequest) {
  return await callAuthedCommand<PageResult<AssetItemRecord>>(
    'asset_item_page',
    {
      request,
    },
  );
}

export interface AssetItemImportResult {
  imported: number;
  scanned: number;
  skipped: number;
  unchanged: number;
  updated: number;
}

export interface AssetItemDeployPreview {
  exists: boolean;
  hasConflict: boolean;
  id: string;
  libraryContent: string;
  localContent: string;
  name: string;
  targetPath: string;
  targetRelativePath: string;
}

export interface AssetVariableRefreshResult {
  candidates: number;
  inserted: number;
  protected: number;
  scanned: number;
  unchanged: number;
  updated: number;
}

export async function assetItemImportDirectoryApi(
  request: { rootPath: string } & Pick<AssetItemPageRequest, 'kind'>,
) {
  return await callAuthedCommand<AssetItemImportResult>(
    'asset_item_import_directory',
    { request },
  );
}

export async function assetItemCreateApi(input: AssetItemInput) {
  return await callAuthedCommand<AssetItemRecord>('asset_item_create', {
    input,
  });
}

export async function assetItemUpdateApi(input: AssetItemUpdateInput) {
  return await callAuthedCommand<AssetItemRecord>('asset_item_update', {
    input,
  });
}

export async function assetItemDeleteApi(id: string) {
  return await callAuthedCommand<null>('asset_item_delete', { id });
}

export async function assetItemToggleApi(id: string, status: string) {
  return await callAuthedCommand<AssetItemRecord>('asset_item_toggle', {
    input: { id, status },
  });
}

export async function assetItemDeployPreviewApi(request: {
  id: string;
  rootPath: string;
}) {
  return await callAuthedCommand<AssetItemDeployPreview>(
    'asset_item_deploy_preview',
    { request },
  );
}

export async function assetItemDeploySaveApi(input: {
  content: string;
  id: string;
  rootPath: string;
}) {
  return await callAuthedCommand<AssetItemRecord>('asset_item_deploy_save', {
    input,
  });
}

export async function assetVariablePageApi(request: AssetVariablePageRequest) {
  return await callAuthedCommand<PageResult<AssetVariableRecord>>(
    'asset_variable_page',
    { request },
  );
}

export async function assetVariableUpsertApi(input: AssetVariableInput) {
  return await callAuthedCommand<AssetVariableRecord>('asset_variable_upsert', {
    input,
  });
}

export async function assetVariableDeleteApi(id: string) {
  return await callAuthedCommand<null>('asset_variable_delete', { id });
}

export async function assetVariableRefreshPageGlobalsApi() {
  return await callAuthedCommand<AssetVariableRefreshResult>(
    'asset_variable_refresh_page_globals',
  );
}

export async function pluginRegistrySnapshotApi() {
  return await callAuthedCommand<PluginRegistrySnapshot>(
    'plugin_registry_snapshot',
  );
}

export async function pluginRegistryReloadApi() {
  return await callAuthedCommand<PluginRegistrySnapshot>(
    'plugin_registry_reload',
  );
}

export async function pluginRegistryLocalStateApi() {
  return await callAuthedCommand<PluginRegistryLocalState>(
    'plugin_registry_local_state',
  );
}

export async function pluginChildCapabilityApproveApi(
  input: ChildCapabilityApprovalInput,
) {
  return await callAuthedCommand<ChildCapabilityApprovalRecord>(
    'plugin_child_capability_approve',
    { input },
  );
}

export async function pluginChildCapabilityRevokeApi(
  input: ChildCapabilityApprovalInput,
) {
  return await callAuthedCommand<ChildCapabilityApprovalRecord>(
    'plugin_child_capability_revoke',
    { input },
  );
}

export async function pluginRegistryInstallApi(sourcePath: string) {
  return await callAuthedCommand<PluginRegistryInstalledRecord>(
    'plugin_registry_install',
    { sourcePath },
  );
}

export async function pluginRegistryEnableApi(id: string) {
  return await callAuthedCommand<PluginRegistryInstalledRecord>(
    'plugin_registry_enable',
    { id },
  );
}

export async function pluginRegistryDisableApi(id: string) {
  return await callAuthedCommand<PluginRegistryInstalledRecord>(
    'plugin_registry_disable',
    { id },
  );
}

export async function pluginRegistryUninstallApi(id: string) {
  return await callAuthedCommand<null>('plugin_registry_uninstall', { id });
}

export async function pluginRegistryRollbackApi(
  input: PluginRegistryRollbackInput,
) {
  return await callAuthedCommand<PluginRegistryRollbackResult>(
    'plugin_registry_rollback',
    { input },
  );
}

export async function appRuntimeSnapshotApi() {
  return await callAuthedCommand<AppRuntimeSnapshot>('app_runtime_snapshot');
}

export async function appRuntimeStartApi(input: AppRuntimeStartInput) {
  return await callAuthedCommand<AppRuntimeLifecycleRecord>(
    'app_runtime_start',
    {
      input,
    },
  );
}

export async function appRuntimeStopApi(input: AppRuntimeStopInput) {
  return await callAuthedCommand<AppRuntimeLifecycleRecord>(
    'app_runtime_stop',
    {
      input,
    },
  );
}

export async function appRuntimeReloadApi(input: AppRuntimeReloadInput) {
  return await callAuthedCommand<AppRuntimeLifecycleRecord>(
    'app_runtime_reload',
    {
      input,
    },
  );
}

export async function appRuntimeWorkspaceApi(input: AppRuntimeWorkspaceInput) {
  return await callAuthedCommand<AppRuntimeLifecycleRecord>(
    'app_runtime_workspace',
    {
      input,
    },
  );
}

export async function appRuntimeSessionApi(input: AppRuntimeSessionInput) {
  return await callAuthedCommand<AppRuntimeLifecycleRecord>(
    'app_runtime_session',
    {
      input,
    },
  );
}

export async function eventBusPublishApi(input: EventBusPublishInput) {
  return await callAuthedCommand<PlatformEventRecord>('event_bus_publish', {
    input,
  });
}

export async function eventBusStreamApi(input: EventBusStreamInput) {
  return await callAuthedCommand<PlatformEventRecord>('event_bus_stream', {
    input,
  });
}

export async function eventBusSnapshotApi(
  request: EventBusSnapshotRequest = {},
) {
  return await callAuthedCommand<PlatformEventRecord[]>('event_bus_snapshot', {
    request,
  });
}

export async function pluginCreateFromPromptApi(
  input: PluginCreateFromPromptInput,
) {
  return await callAuthedCommand<PluginDraftCreationResult>(
    'plugin_create_from_prompt',
    { input },
  );
}

export async function pluginPublishLocalApi(sourcePath: string) {
  return await callAuthedCommand<PluginRegistryInstalledRecord>(
    'plugin_publish_local',
    { sourcePath },
  );
}

export async function pluginPublishGateApi(sourcePath: string, write = true) {
  return await callAuthedCommand<PluginPublishGateReport>(
    'plugin_publish_gate',
    { sourcePath, write },
  );
}

export async function pluginRepairFromDiagnosticsApi(
  input: PluginRepairFromDiagnosticsInput,
) {
  return await callAuthedCommand<PluginDraftCreationResult>(
    'plugin_repair_from_diagnostics',
    { input },
  );
}

export async function pluginVerifyDraftApi(input: PluginVerifyDraftInput) {
  return await callAuthedCommand<PluginVerificationReport>(
    'plugin_verify_draft',
    { input },
  );
}

export async function capabilityAuditLogApi() {
  return await callAuthedCommand<CapabilityAuditRecord[]>(
    'capability_audit_log',
  );
}

export async function permissionAuditLogApi() {
  return await callAuthedCommand<PermissionDecisionRecord[]>(
    'permission_audit_log',
  );
}

export async function permissionConsentListApi() {
  return await callAuthedCommand<PermissionConsentRecord[]>(
    'permission_consent_list',
  );
}

export async function permissionConsentGrantApi(
  input: PermissionConsentGrantInput,
) {
  return await callAuthedCommand<PermissionConsentRecord>(
    'permission_consent_grant',
    { input },
  );
}

export async function permissionConsentRevokeApi(
  input: PermissionConsentRevokeInput,
) {
  return await callAuthedCommand<PermissionConsentRecord>(
    'permission_consent_revoke',
    { input },
  );
}

export async function permissionApprovalListApi() {
  return await callAuthedCommand<PermissionApprovalRecord[]>(
    'permission_approval_list',
  );
}

export async function permissionApprovalApproveApi(
  input: PermissionApprovalDecisionInput,
) {
  return await callAuthedCommand<PermissionApprovalRecord>(
    'permission_approval_approve',
    { input },
  );
}

export async function permissionApprovalDenyApi(
  input: PermissionApprovalDecisionInput,
) {
  return await callAuthedCommand<PermissionApprovalRecord>(
    'permission_approval_deny',
    { input },
  );
}

export async function capabilityClipboardWriteApi(input: { text: string }) {
  return await callAuthedCommand<number>('capability_clipboard_write', {
    input,
  });
}

export async function capabilityClipboardReadApi() {
  return await callAuthedCommand<string>('capability_clipboard_read');
}

export async function capabilityNotificationSendApi(input: {
  body?: string;
  title: string;
}) {
  return await callAuthedCommand<null>('capability_notification_send', {
    input,
  });
}

export async function capabilityFsReadApi(input: FsReadInput) {
  return await callAuthedCommand<FsReadResult>('capability_fs_read', {
    input,
  });
}

export async function capabilityFsWriteApi(input: FsWriteInput) {
  return await callAuthedCommand<FsWriteResult>('capability_fs_write', {
    input,
  });
}

export async function capabilityProcessExecApi(input: ProcessExecInput) {
  return await callAuthedCommand<ProcessExecResult>('capability_process_exec', {
    input,
  });
}

export async function capabilityBrowserOpenUrlApi(input: BrowserOpenUrlInput) {
  return await callAuthedCommand<string>('capability_browser_open_url', {
    input,
  });
}

export async function capabilityInvokeApi<T = unknown>(
  input: CapabilityInvokeInput,
) {
  return await callAuthedCommand<T>('capability_invoke', { input });
}

export async function pluginHostLoadApi(input: ExtensionHostSourceInput) {
  return await callAuthedCommand<ExtensionHostPluginRecord>(
    'plugin_host_load',
    { input },
  );
}

export async function pluginHostActivateApi(pluginId: string) {
  return await callAuthedCommand<ExtensionHostPluginRecord>(
    'plugin_host_activate',
    { pluginId },
  );
}

export async function pluginHostDeactivateApi(pluginId: string) {
  return await callAuthedCommand<ExtensionHostPluginRecord>(
    'plugin_host_deactivate',
    { pluginId },
  );
}

export async function pluginHostReloadApi(input: ExtensionHostSourceInput) {
  return await callAuthedCommand<ExtensionHostPluginRecord>(
    'plugin_host_reload',
    { input },
  );
}

export async function pluginHostDisposeApi(pluginId: string) {
  return await callAuthedCommand<ExtensionHostPluginRecord>(
    'plugin_host_dispose',
    { pluginId },
  );
}

export async function pluginHostSnapshotApi() {
  return await callAuthedCommand<ExtensionHostPluginRecord[]>(
    'plugin_host_snapshot',
  );
}

export interface OpenAIAssistantPageContextInput {
  html?: string;
  selection?: string;
  text?: string;
  title?: string;
  url?: string;
}

export interface OpenAIAssistantPageContextPreview {
  characterCount: number;
  content: string;
  description: null | string;
  source: string;
  title: string;
  truncated: boolean;
  url: null | string;
}

export type OpenAIAssistantRole = 'assistant' | 'user';

export interface OpenAIAssistantTurn {
  content: string;
  role: OpenAIAssistantRole;
}

export interface OpenAIAssistantChatRequest {
  context: OpenAIAssistantPageContextInput;
  history?: OpenAIAssistantTurn[];
  model?: string;
  question: string;
}

export interface OpenAIAssistantChatResponse {
  answer: string;
  context: OpenAIAssistantPageContextPreview;
  model: string;
  responseId: null | string;
}

export async function openAIAssistantPreviewContextApi(
  input: OpenAIAssistantPageContextInput,
) {
  return await callAuthedCommand<OpenAIAssistantPageContextPreview>(
    'openai_assistant_preview_context',
    { input },
  );
}

export async function openAIAssistantChatApi(
  input: OpenAIAssistantChatRequest,
) {
  return await callAuthedCommand<OpenAIAssistantChatResponse>(
    'openai_assistant_chat',
    { input },
  );
}

export async function openAppDataDirApi() {
  return await callAuthedCommand<string>('app_open_data_dir');
}
