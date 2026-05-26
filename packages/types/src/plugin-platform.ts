export interface PluginPlatformMatrix {
  degraded?: string[];
  reason?: string;
  supported?: string[];
  unsupported?: string[];
}

export interface PluginCapabilityFormula {
  allow?: string[];
  id: string;
  optional?: boolean;
  platforms?: string[];
  reason?: string;
  scope?: string;
}

export type PluginTrustLevel =
  | 'community'
  | 'platform'
  | 'trusted-provider'
  | 'verified';

export type PluginCapabilityProviderKind =
  | 'native'
  | 'remote'
  | 'service'
  | 'web';

export interface PluginCapabilityProviderContribution {
  capability: string;
  entry?: string;
  fallback?: string;
  id: string;
  kind?: PluginCapabilityProviderKind | string;
  platforms?: string[];
  title?: string;
  trustLevel: PluginTrustLevel | string;
  when?: string;
}

export interface PluginCommandContribution {
  capabilities?: string[];
  category?: string;
  id: string;
  title: string;
  when?: string;
}

export interface PluginToolContribution {
  capabilities?: string[];
  category?: string;
  id: string;
  input?: string;
  output?: string;
  title?: string;
  when?: string;
}

export interface PluginMenuContribution {
  code: string;
  component?: string;
  icon?: string;
  ordinaryUser?: boolean;
  parentPermissionId?: null | string;
  path?: string;
  permissionId?: string;
  permissionType?: 'button' | 'menu';
  sortOrder?: number;
  title: string;
}

export interface PluginPermissionContribution {
  code: string;
  component?: string;
  icon?: string;
  ordinaryUser?: boolean;
  parentPermissionId?: null | string;
  path?: string;
  permissionId?: string;
  permissionType?: 'button' | 'menu';
  sortOrder?: number;
  title: string;
}

export type PluginUiSchemaKind =
  | 'detail'
  | 'form'
  | 'graph'
  | 'markdown'
  | 'summary-list'
  | 'table'
  | 'timeline'
  | 'tree'
  | 'wizard';

export interface PluginViewContribution {
  assetItemKind?: string;
  contract?: string;
  id: string;
  path?: string;
  schema: PluginUiSchemaKind;
  slot?: string;
  when?: string;
}

export interface PluginUiKeyValueItem {
  hint?: string;
  label: string;
  value: string;
}

export interface PluginUiSummaryListSchema {
  items: PluginUiKeyValueItem[];
  kind: 'summary-list';
  title?: string;
}

export interface PluginUiDetailSchema {
  items: PluginUiKeyValueItem[];
  kind: 'detail';
  title?: string;
}

export interface PluginUiTableColumn {
  align?: 'center' | 'left' | 'right';
  key: string;
  label: string;
  width?: number | string;
}

export interface PluginUiTableSchema {
  columns: PluginUiTableColumn[];
  kind: 'table';
  rows: Array<Record<string, unknown>>;
  title?: string;
}

export interface PluginUiTreeNode {
  children?: PluginUiTreeNode[];
  id: string;
  label: string;
  value?: string;
}

export interface PluginUiTreeSchema {
  kind: 'tree';
  nodes: PluginUiTreeNode[];
  title?: string;
}

export interface PluginUiGraphNode {
  group?: string;
  id: string;
  label: string;
  value?: string;
}

export interface PluginUiGraphEdge {
  from: string;
  label?: string;
  to: string;
}

export interface PluginUiGraphSchema {
  edges: PluginUiGraphEdge[];
  kind: 'graph';
  nodes: PluginUiGraphNode[];
  title?: string;
}

export interface PluginUiFormField {
  hint?: string;
  label: string;
  required?: boolean;
  value: string;
}

export interface PluginUiFormSchema {
  fields: PluginUiFormField[];
  kind: 'form';
  submitLabel?: string;
  title?: string;
}

export interface PluginUiTimelineItem {
  label: string;
  time?: string;
  tone?: 'danger' | 'info' | 'success' | 'warning';
  value: string;
}

export interface PluginUiTimelineSchema {
  items: PluginUiTimelineItem[];
  kind: 'timeline';
  title?: string;
}

export interface PluginUiMarkdownSchema {
  content: string;
  kind: 'markdown';
  title?: string;
}

export interface PluginUiWizardStep {
  description?: string;
  id: string;
  title: string;
}

export interface PluginUiWizardSchema {
  activeStep?: string;
  kind: 'wizard';
  steps: PluginUiWizardStep[];
  title?: string;
}

export type PluginUiRenderSchema =
  | PluginUiDetailSchema
  | PluginUiFormSchema
  | PluginUiGraphSchema
  | PluginUiMarkdownSchema
  | PluginUiSummaryListSchema
  | PluginUiTableSchema
  | PluginUiTimelineSchema
  | PluginUiTreeSchema
  | PluginUiWizardSchema;

export interface PluginExtensionPointContribution {
  activation?: string;
  allowedContributionKinds?: string[];
  contract?: string;
  id: string;
  multiplicity?: string;
  title: string;
}

export interface PluginSettingContribution {
  id: string;
  schema?: string;
  title?: string;
}

export interface PluginResourceContribution {
  id: string;
  kind?: string;
  schema?: string;
  title?: string;
}

export interface PluginRouteContribution {
  component?: string;
  id: string;
  path: string;
  slot?: string;
  title?: string;
  when?: string;
}

export interface PluginPolicyMatchContribution {
  capabilities?: string[];
  platforms?: string[];
  scopes?: string[];
  sourceIds?: string[];
  sourceKinds?: string[];
  targetContains?: string[];
}

export interface PluginPolicyContribution {
  effect?: 'deny' | 'warn' | string;
  id: string;
  matches?: PluginPolicyMatchContribution;
  priority?: number;
  reason?: string;
  title: string;
  when?: string;
}

export interface PluginContributes {
  capabilityProviders?: PluginCapabilityProviderContribution[];
  commands?: PluginCommandContribution[];
  extensionPoints?: PluginExtensionPointContribution[];
  menus?: PluginMenuContribution[];
  permissions?: PluginPermissionContribution[];
  policies?: PluginPolicyContribution[];
  resources?: PluginResourceContribution[];
  routes?: PluginRouteContribution[];
  settings?: PluginSettingContribution[];
  tools?: PluginToolContribution[];
  views?: PluginViewContribution[];
}

export interface PluginParent {
  compatibleParentRange?: string;
  mount: string;
  pluginId: string;
}

export interface PluginEntry {
  browser?: string;
  node?: string;
  remote?: string;
  worker?: string;
}

export interface PluginEventDeclaration {
  event: string;
  schema: string;
}

export interface PluginEvents {
  publishes?: Array<PluginEventDeclaration | string>;
  subscribes?: Array<PluginEventDeclaration | string>;
}

export interface PluginAiHints {
  doNotSplitBelow?: string;
  editableByAI?: boolean;
  preferredFiles?: string[];
  primaryEditFile?: string;
  primaryOutput?: string;
  repairStrategy?: string;
  role?: string;
}

export interface PluginChildPolicy {
  allowThirdPartyChildren?: boolean;
  allowedChildCapabilities?: string[];
  capabilityMode?: 'explicit-escalation' | 'inherit' | 'intersection' | string;
  eventNamespace?: string;
  requiresPlatformApprovalForNewCapabilities?: boolean;
}

export interface PluginFormulaContract {
  activation?: string[];
  ai?: PluginAiHints;
  capabilities?: PluginCapabilityFormula[];
  childPolicy?: PluginChildPolicy;
  contributes?: PluginContributes;
  displayName: string;
  entry?: PluginEntry;
  events?: PluginEvents;
  expectations?: string[];
  id: string;
  intent?: string;
  kind: 'child-plugin' | 'plugin';
  parent?: null | PluginParent;
  platforms?: PluginPlatformMatrix;
  schemaVersion: 'plugin-formula/v1';
  trustLevel?: PluginTrustLevel | string;
  version?: string;
}

export interface SystemProvides {
  commands?: string[];
  tools?: string[];
}

export interface SystemCapsuleContract {
  ai?: PluginAiHints;
  capabilities?: PluginCapabilityFormula[];
  contributes?: PluginContributes;
  displayName: string;
  id: string;
  intent?: string;
  kind: 'system';
  managedBy?: string;
  mutable?: boolean;
  childPolicy?: PluginChildPolicy;
  platforms?: PluginPlatformMatrix;
  provides?: SystemProvides;
  replaceable?: boolean;
  schemaVersion: 'system-capsule/v1';
  trustLevel?: PluginTrustLevel | string;
}

export interface SeedPermission {
  code: string;
  component: string;
  icon: string;
  id: string;
  name: string;
  ordinaryUser: boolean;
  parentId?: null | string;
  path: string;
  permissionType: 'button' | 'menu';
  sortOrder: number;
}

export interface PluginRegistrySnapshot<
  TCommand = string,
  TPermission = string,
> {
  commands: Array<{
    capabilities?: string[];
    category?: string;
    id: TCommand;
    parentChain: string[];
    sourceId: string;
    sourceKind: string;
    title: string;
    when: string;
  }>;
  capabilities: Array<{
    allow: string[];
    id: string;
    optional: boolean;
    parentChain: string[];
    platforms: string[];
    reason: string;
    scope: string;
    sourceId: string;
    sourceKind: string;
  }>;
  capabilityProviders: Array<{
    capability: string;
    entry: string;
    fallback: string;
    id: string;
    kind: PluginCapabilityProviderKind | string;
    parentChain: string[];
    platforms: string[];
    sourceId: string;
    sourceKind: string;
    sourceTrustLevel: PluginTrustLevel | string;
    title: string;
    trustLevel: PluginTrustLevel | string;
    when: string;
  }>;
  policies: Array<{
    capabilities: string[];
    effect: string;
    id: string;
    parentChain: string[];
    platforms: string[];
    priority: number;
    reason: string;
    scopes: string[];
    sourceId: string;
    sourceIds: string[];
    sourceKind: string;
    sourceKinds: string[];
    targetContains: string[];
    title: string;
    when: string;
  }>;
  diagnostics: string[];
  extensionTree: {
    mounts: Array<{
      capabilities: string[];
      capabilityEscalations: string[];
      childPluginId: string;
      commands: string[];
      compatibleParentRange: string;
      effectiveCapabilities: string[];
      extensionPointId: string;
      menus: string[];
      parentChain: string[];
      parentPluginId: string;
      routes: string[];
      views: string[];
    }>;
    nodes: Array<{
      capabilities: string[];
      capabilityEscalations: string[];
      childCount: number;
      commands: string[];
      depth: number;
      displayName: string;
      effectiveCapabilities: string[];
      extensionPoints: string[];
      kind: string;
      menus: string[];
      parentChain: string[];
      parentMount?: null | string;
      parentPluginId?: null | string;
      pluginId: string;
      routes: string[];
      version: string;
      views: string[];
    }>;
  };
  extensionPoints: Array<{
    activation: string;
    allowedContributionKinds: string[];
    contract: string;
    id: string;
    multiplicity: string;
    parentChain: string[];
    sourceId: string;
    sourceKind: string;
    title: string;
  }>;
  events: Array<{
    direction: 'publish' | 'subscribe' | string;
    event: string;
    parentChain: string[];
    schema: string;
    sourceId: string;
    sourceKind: string;
  }>;
  permissions: Array<{
    code: TPermission;
    component: string;
    icon: string;
    ordinaryUser: boolean;
    parentChain: string[];
    parentPermissionId?: null | string;
    path: string;
    permissionId: string;
    permissionType: 'button' | 'menu';
    sortOrder: number;
    sourceId: string;
    sourceKind: string;
    title: string;
  }>;
  permissionSeeds: SeedPermission[];
  plugins: Array<{
    capabilities: string[];
    capabilityProviders: string[];
    commands: string[];
    displayName: string;
    extensionPoints: string[];
    id: string;
    intent: string;
    kind: string;
    menus: string[];
    parentMount?: null | string;
    parentPluginId?: null | string;
    platformDegraded: string[];
    platformSupported: string[];
    platformUnsupported: string[];
    resources: string[];
    routes: string[];
    settings: string[];
    tools: string[];
    trustLevel: PluginTrustLevel | string;
    version: string;
    views: string[];
  }>;
  resources: Array<{
    id: string;
    kind: string;
    parentChain: string[];
    schema: string;
    sourceId: string;
    sourceKind: string;
    title: string;
  }>;
  routes: Array<{
    component: string;
    id: string;
    parentChain: string[];
    path: string;
    slot: string;
    sourceId: string;
    sourceKind: string;
    title: string;
    when: string;
  }>;
  schemaVersion: string;
  settings: Array<{
    id: string;
    parentChain: string[];
    schema: string;
    sourceId: string;
    sourceKind: string;
    title: string;
  }>;
  systemCapsules: Array<{
    capabilities: string[];
    capabilityProviders: string[];
    commands: string[];
    displayName: string;
    extensionPoints: string[];
    id: string;
    intent: string;
    kind: string;
    menus: string[];
    parentMount?: null | string;
    parentPluginId?: null | string;
    platformDegraded: string[];
    platformSupported: string[];
    platformUnsupported: string[];
    resources: string[];
    routes: string[];
    settings: string[];
    tools: string[];
    trustLevel: PluginTrustLevel | string;
    version: string;
    views: string[];
  }>;
  tools: Array<{
    capabilities: string[];
    category: string;
    id: string;
    input: string;
    output: string;
    parentChain: string[];
    sourceId: string;
    sourceKind: string;
    title: string;
    when: string;
  }>;
  views: Array<{
    contract: string;
    id: string;
    parentChain: string[];
    path: string;
    schema: PluginUiSchemaKind;
    slot: string;
    sourceId: string;
    sourceKind: string;
    when: string;
  }>;
}

export interface CapabilityAuditRecord {
  action: string;
  capability: string;
  detail?: null | string;
  outcome: string;
  target: string;
  traceId: string;
}

export interface PermissionDecisionRecord {
  capability: string;
  decision: string;
  reason: string;
  scope: string;
  sourceId: string;
  sourceKind: string;
  target: string;
  timestamp: number;
  traceId: string;
  userId: string;
}

export interface PermissionRuntimeRequest {
  capability: string;
  consentGranted?: boolean;
  declaredCapabilities?: string[];
  declaredScopes?: string[];
  platformDegraded?: string[];
  platformSupported?: string[];
  platformUnsupported?: string[];
  reason?: string;
  sourceId: string;
  sourceKind: string;
  scope?: string;
  target: string;
  userId: string;
}

export interface PermissionConsentRecord {
  capability: string;
  createdAt: number;
  id: string;
  reason: string;
  scope: string;
  sourceId: string;
  sourceKind: string;
  status: string;
  updatedAt: number;
  userId: string;
}

export interface PermissionConsentGrantInput {
  capability: string;
  reason?: string;
  scope?: string;
  sourceId: string;
  sourceKind: string;
}

export interface PermissionConsentRevokeInput {
  capability: string;
  scope?: string;
  sourceId: string;
}

export interface PermissionApprovalRecord {
  capability: string;
  createdAt: number;
  decisionReason: string;
  decidedAt?: null | number;
  id: string;
  reason: string;
  scope: string;
  sourceId: string;
  sourceKind: string;
  status: string;
  target: string;
  updatedAt: number;
  userId: string;
}

export interface PermissionApprovalRequestInput {
  capability: string;
  reason?: string;
  scope?: string;
  sourceId: string;
  sourceKind?: string;
  target?: string;
}

export interface PermissionApprovalDecisionInput {
  id: string;
  reason?: string;
}

export interface PlatformEventRecord {
  correlationId?: null | string;
  done?: boolean;
  id: string;
  kind: 'error' | 'publish' | 'reply' | 'request' | string;
  parentTraceId?: null | string;
  payload: unknown;
  permissions?: null | unknown;
  schema: string;
  sequence?: null | number;
  source: string;
  target?: null | string;
  timestamp: number;
  traceId: string;
  type: string;
}

export interface EventBusPublishInput {
  eventType: string;
  parentTraceId?: null | string;
  payload: unknown;
  permissions?: null | unknown;
  schema?: null | string;
  source: string;
  target?: null | string;
}

export interface EventBusStreamInput {
  done?: boolean;
  eventType: string;
  parentTraceId?: null | string;
  payload: unknown;
  permissions?: null | unknown;
  schema?: null | string;
  sequence?: null | number;
  source: string;
  streamId?: null | string;
  target?: null | string;
}

export interface EventBusSnapshotRequest {
  eventType?: null | string;
  limit?: null | number;
}

export interface AppRuntimeLifecycleRecord {
  action: string;
  dataDir: string;
  mode: string;
  platformId: string;
  reason: string;
  reloadCount: number;
  sessionId?: null | string;
  status: string;
  timestamp: number;
  workspace?: null | string;
}

export interface AppRuntimeSnapshot {
  dataDir: string;
  lifecycle: AppRuntimeLifecycleRecord[];
  mode: string;
  platformId: string;
  reloadCount: number;
  schemaVersion: 'app-runtime/v1' | string;
  sessionId?: null | string;
  startedAt?: null | number;
  status: string;
  stoppedAt?: null | number;
  updatedAt: number;
  workspace?: null | string;
}

export interface AppRuntimeStartInput {
  mode?: null | string;
  reason?: null | string;
  sessionId?: null | string;
  workspace?: null | string;
}

export interface AppRuntimeStopInput {
  reason?: null | string;
}

export interface AppRuntimeReloadInput {
  reason?: null | string;
  sessionId?: null | string;
  workspace?: null | string;
}

export interface AppRuntimeWorkspaceInput {
  reason?: null | string;
  workspace: string;
}

export interface AppRuntimeSessionInput {
  reason?: null | string;
  sessionId: string;
}

export interface ClipboardWriteInput {
  text: string;
}

export interface NotificationSendInput {
  body?: string;
  title: string;
}

export interface FsReadInput {
  maxBytes?: null | number;
  path: string;
}

export interface FsReadResult {
  bytes: number;
  content: string;
  path: string;
  truncated: boolean;
}

export interface FsWriteInput {
  append?: boolean;
  content: string;
  createDirs?: boolean;
  path: string;
}

export interface FsWriteResult {
  bytes: number;
  path: string;
}

export interface ProcessExecInput {
  args?: string[];
  command: string;
  cwd?: null | string;
}

export interface ProcessExecResult {
  code?: null | number;
  command: string;
  stderr: string;
  stderrTruncated: boolean;
  stdout: string;
  stdoutTruncated: boolean;
  success: boolean;
}

export interface BrowserOpenUrlInput {
  url: string;
}

export interface CapabilityInvokeInput {
  capability: string;
  input?: unknown;
}

export interface PluginRegistryInstalledRecord {
  contentHash: string;
  enabled: boolean;
  id: string;
  installedAt: number;
  kind: string;
  registryPath: string;
  schemaVersion: string;
  signaturePath?: string;
  sourcePath: string;
  updatedAt: number;
  version?: string;
}

export interface PluginRegistryLockRecord {
  contentHash: string;
  enabled: boolean;
  id: string;
  lockedAt: number;
  registryPath: string;
  schemaVersion: string;
  signaturePath?: string;
  sourcePath: string;
  updatedAt: number;
  version?: string;
}

export interface PluginRegistryAuditRecord {
  action: string;
  contentHash?: null | string;
  detail?: null | string;
  id: string;
  path?: null | string;
  status: string;
  timestamp: number;
}

export interface PluginPackageSignaturePlaceholder {
  algorithm: string;
  contentHash: string;
  createdAt: number;
  keyId: string;
  pluginId: string;
  reason: string;
  schemaVersion: 'plugin-signature-placeholder/v1';
  signature: string;
  status: 'placeholder' | string;
}

export interface PluginRegistryVersionRecord {
  contentHash: string;
  createdAt: number;
  id: string;
  kind: string;
  registryPath: string;
  schemaVersion: string;
  signaturePath: string;
  sourcePath: string;
  version: string;
}

export interface ChildCapabilityApprovalRecord {
  capability: string;
  childPluginId: string;
  createdAt: number;
  parentPluginId: string;
  reason: string;
  revokedAt?: null | number;
  revokedReason: string;
  status: 'approved' | 'revoked' | string;
  updatedAt: number;
}

export interface ChildCapabilityApprovalInput {
  capability: string;
  childPluginId: string;
  parentPluginId: string;
  reason?: string;
}

export interface PluginRegistryRollbackInput {
  contentHash?: string;
  id: string;
}

export interface PluginRegistryRollbackResult {
  previousContentHash: string;
  restored: PluginRegistryInstalledRecord;
  selectedHistory: PluginRegistryVersionRecord;
}

export interface PluginRegistryLocalState {
  audits: PluginRegistryAuditRecord[];
  childCapabilityApprovals: ChildCapabilityApprovalRecord[];
  history: PluginRegistryVersionRecord[];
  installed: PluginRegistryInstalledRecord[];
  locks: PluginRegistryLockRecord[];
}

export interface PluginCreateFromPromptInput {
  displayName?: string;
  force?: boolean;
  id?: string;
  kind?: 'child-plugin' | 'plugin';
  outputDir?: string;
  parentMount?: string;
  parentPluginId?: string;
  prompt: string;
  routePath?: string;
}

export interface PluginPermissionPlanItem {
  allow: string[];
  id: string;
  optional: boolean;
  reason: string;
  scope: string;
}

export interface PluginDraft {
  diagnostics: string[];
  formula: PluginFormulaContract;
  generatedFiles: string[];
  permissionPlan: PluginPermissionPlanItem[];
  prompt: string;
  schemaVersion: 'plugin-draft/v1';
}

export interface PluginDraftWriteResult {
  files: string[];
  outputDir: string;
}

export interface PluginDraftCreationResult {
  draft: PluginDraft;
  writeResult?: null | PluginDraftWriteResult;
}

export interface PluginDiagnosticPermissionError {
  capability: string;
  reason: string;
}

export interface PluginDiagnosticPlatformError {
  platform: 'linux' | 'macos' | 'remote' | 'web' | 'windows';
  reason: string;
}

export interface PluginDiagnosticDomSnapshot {
  html: string;
  id: string;
  path?: string;
}

export interface PluginDiagnosticUiPreview {
  domSnapshots?: PluginDiagnosticDomSnapshot[];
  screenshots?: string[];
}

export interface PluginDiagnosticsPackage {
  formulaErrors: string[];
  permissionErrors: PluginDiagnosticPermissionError[];
  platformErrors: PluginDiagnosticPlatformError[];
  pluginId: string;
  repairHint: string;
  runId: string;
  sourcePath?: string;
  status: 'failed' | 'passed' | 'warning';
  testFailures: string[];
  uiPreview?: PluginDiagnosticUiPreview;
}

export interface PluginRepairFromDiagnosticsInput {
  diagnosticsPath: string;
  force?: boolean;
  sourcePath?: string;
}

export interface PluginVerifyDraftInput {
  outputDir?: string;
  sourcePath: string;
  write?: boolean;
}

export interface PluginVerificationCheck {
  detail?: string;
  id: string;
  status: 'failed' | 'passed' | 'warning';
  title: string;
}

export interface PluginVerificationReport {
  checks: PluginVerificationCheck[];
  diagnostics: PluginDiagnosticsPackage;
  pluginId: string;
  runId: string;
  schemaVersion: 'plugin-verification/v1';
  status: 'failed' | 'passed' | 'warning';
  writtenFiles: string[];
}

export interface PluginPublishGateReport {
  audit: PluginRegistryAuditRecord;
  checks: PluginVerificationCheck[];
  contentHash: string;
  lock: PluginRegistryLockRecord;
  pluginId: string;
  remoteRegistryProtocolPath: string;
  runId: string;
  schemaVersion: 'plugin-publish-gate/v1';
  signature: PluginPackageSignaturePlaceholder;
  status: 'failed' | 'passed' | 'warning';
  verification: PluginVerificationReport;
  writtenFiles: string[];
}

export interface ExtensionHostSourceInput {
  sourcePath: string;
}

export interface ExtensionHostApiSnapshot {
  action: 'activate' | 'deactivate' | 'dispose' | 'load' | 'reload' | string;
  entryPath: string;
  hostKind: string;
  pluginId: string;
  schemaVersion: 'extension-host-api/v1';
  sourcePath: string;
  supportedCapabilities: string[];
}

export interface ExtensionHostApiInvocationResult {
  capability: string;
  hostKind: string;
  input: unknown;
  pluginId: string;
  reason: string;
  status: 'unsupported' | string;
}

export interface ExtensionHostApiEventResult {
  eventType: string;
  hostKind: string;
  payload: unknown;
  pluginId: string;
  sequence?: number;
  status: 'recorded' | string;
}

export interface PluginHostApi {
  capabilities: {
    invoke(
      capability: string,
      input?: unknown,
    ): ExtensionHostApiInvocationResult;
  };
  events: {
    publish(eventType: string, payload?: unknown): ExtensionHostApiEventResult;
    request(eventType: string, payload?: unknown): ExtensionHostApiEventResult;
    stream(
      eventType: string,
      payload?: unknown,
      sequence?: number,
      done?: boolean,
    ): ExtensionHostApiEventResult;
  };
  host: {
    describe(): ExtensionHostApiSnapshot;
    snapshot(): ExtensionHostApiSnapshot;
  };
  schemaVersion: 'extension-host-api/v1';
}

export interface ExtensionHostPluginRecord {
  activatedAt?: null | number;
  deactivatedAt?: null | number;
  displayName: string;
  disposedAt?: null | number;
  entryPath: string;
  hostKind: 'browser' | 'metadata-only' | 'node' | 'remote' | 'worker' | string;
  lastError?: null | string;
  loadedAt: number;
  logs: string[];
  pluginId: string;
  reloadCount: number;
  sourcePath: string;
  state: 'activated' | 'deactivated' | 'disposed' | 'error' | 'loaded' | string;
}

export interface PluginContext {
  api?: PluginHostApi;
  hostKind?: string;
  log?: (message: string) => void;
  pluginId?: string;
}

export interface PluginCompileResult {
  diagnostics: string[];
  permissions: SeedPermission[];
}

export function compilePluginSnapshot(
  snapshot: Pick<PluginRegistrySnapshot, 'diagnostics' | 'permissionSeeds'>,
): PluginCompileResult {
  return {
    diagnostics: [...snapshot.diagnostics],
    permissions: snapshot.permissionSeeds.map((permission) => ({
      ...permission,
    })),
  };
}
