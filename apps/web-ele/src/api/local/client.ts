import { useAccessStore } from '@vben/stores';

import { invoke } from '@tauri-apps/api/core';
import { ElMessage } from 'element-plus';

interface CommandError {
  code: number;
  details?: string;
  message: string;
}

function getToken() {
  const accessStore = useAccessStore();
  return accessStore.accessToken ?? '';
}

function getRuntimeCommandBridgeUrl() {
  if (typeof window === 'undefined') {
    return '';
  }
  const runtimeWindow = window as {
    __AIO_COMMAND_BRIDGE_URL__?: string;
  } & Window;
  return (
    window.localStorage.getItem('AIO_COMMAND_BRIDGE_URL') ||
    runtimeWindow.__AIO_COMMAND_BRIDGE_URL__ ||
    ''
  );
}

const commandBridgeUrl =
  getRuntimeCommandBridgeUrl() ||
  import.meta.env.VITE_AIO_COMMAND_BRIDGE_URL ||
  (import.meta.env.DEV ? 'http://127.0.0.1:18777/__aio/command' : '');

const WEB_RUNTIME_STATE_KEY = 'AIO_WEB_RUNTIME_STATE';

function hasTauriInvoke() {
  if (typeof window === 'undefined') {
    return false;
  }
  const internals = (
    window as { __TAURI_INTERNALS__?: { invoke?: unknown } } & Window
  ).__TAURI_INTERNALS__;
  return typeof internals?.invoke === 'function';
}

function normalizeError(error: unknown): CommandError {
  if (typeof error === 'object' && error !== null && 'message' in error) {
    const commandError = error as CommandError;
    return {
      code: commandError.code ?? 500,
      details: commandError.details,
      message: commandError.message,
    };
  }

  return {
    code: 500,
    message: error instanceof Error ? error.message : `${error}`,
  };
}

export async function callCommand<T>(
  command: string,
  payload: Record<string, unknown> = {},
) {
  try {
    if (hasTauriInvoke()) {
      return await invoke<T>(command, payload);
    }
    if (commandBridgeUrl) {
      const response = await fetch(commandBridgeUrl, {
        body: JSON.stringify({
          command,
          payload,
        }),
        headers: {
          'Content-Type': 'application/json',
        },
        method: 'POST',
      });

      if (!response.ok) {
        throw await response.json();
      }

      return (await response.json()) as T;
    }
    const webProvider = await tryWebProviderCommand<T>(command, payload);
    if (webProvider.handled) {
      return webProvider.value as T;
    }
    throw new Error('当前浏览器环境未启用 AIO 本地命令桥，请使用桌面端');
  } catch (error) {
    const commandError = normalizeError(error);
    ElMessage.error(commandError.message);
    throw commandError;
  }
}

async function tryWebProviderCommand<T>(
  command: string,
  payload: Record<string, unknown>,
): Promise<{ handled: boolean; value?: T }> {
  if (command === 'capability_clipboard_write') {
    return {
      handled: true,
      value: (await writeWebClipboard(getInputObject(payload))) as T,
    };
  }
  if (command === 'capability_clipboard_read') {
    return {
      handled: true,
      value: (await readWebClipboard()) as T,
    };
  }
  if (command === 'capability_notification_send') {
    await sendWebNotification(getInputObject(payload));
    return { handled: true, value: null as T };
  }
  if (command === 'app_runtime_snapshot') {
    return { handled: true, value: getWebRuntimeSnapshot() as T };
  }
  if (command === 'app_runtime_start') {
    return {
      handled: true,
      value: updateWebRuntimeState('start', getInputObject(payload)) as T,
    };
  }
  if (command === 'app_runtime_stop') {
    return {
      handled: true,
      value: updateWebRuntimeState('stop', getInputObject(payload)) as T,
    };
  }
  if (command === 'app_runtime_reload') {
    return {
      handled: true,
      value: updateWebRuntimeState('reload', getInputObject(payload)) as T,
    };
  }
  if (command === 'app_runtime_workspace') {
    return {
      handled: true,
      value: updateWebRuntimeState('workspace', getInputObject(payload)) as T,
    };
  }
  if (command === 'app_runtime_session') {
    return {
      handled: true,
      value: updateWebRuntimeState('session', getInputObject(payload)) as T,
    };
  }
  if (command === 'capability_invoke') {
    return await tryWebCapabilityInvoke<T>(getInputObject(payload));
  }
  return { handled: false };
}

async function tryWebCapabilityInvoke<T>(
  input: Record<string, unknown>,
): Promise<{ handled: boolean; value?: T }> {
  const capability =
    typeof input.capability === 'string' ? input.capability : '';
  const capabilityInput = getNestedInputObject(input);
  if (capability === 'clipboard.write') {
    return {
      handled: true,
      value: (await writeWebClipboard(capabilityInput)) as T,
    };
  }
  if (capability === 'clipboard.read') {
    return {
      handled: true,
      value: (await readWebClipboard()) as T,
    };
  }
  if (capability === 'notification.send') {
    await sendWebNotification(capabilityInput);
    return { handled: true, value: null as T };
  }
  return { handled: false };
}

function getInputObject(payload: Record<string, unknown>) {
  return isRecord(payload.input) ? payload.input : {};
}

function getNestedInputObject(input: Record<string, unknown>) {
  return isRecord(input.input) ? input.input : {};
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function getWebRuntimeSnapshot() {
  return readWebRuntimeState();
}

interface WebRuntimeLifecycleRecord {
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

interface WebRuntimeState {
  dataDir: string;
  lifecycle: WebRuntimeLifecycleRecord[];
  mode: string;
  platformId: string;
  reloadCount: number;
  schemaVersion: string;
  sessionId: null | string;
  startedAt: null | number;
  status: string;
  stoppedAt: null | number;
  updatedAt: number;
  workspace: null | string;
}

function updateWebRuntimeState(action: string, input: Record<string, unknown>) {
  const state = readWebRuntimeState();
  const now = Date.now();
  const workspace = pickTrimmedString(input.workspace, state.workspace);
  const sessionId = pickTrimmedString(input.sessionId, state.sessionId);
  const mode = pickTrimmedString(input.mode, state.mode) || state.mode || 'web';
  const reason = pickTrimmedString(input.reason, '') || '';

  if (action === 'stop') {
    state.status = 'stopped';
  } else if (action === 'start' || action === 'reload') {
    state.status = 'running';
  }
  if (action === 'reload') {
    state.reloadCount += 1;
  }
  if (action === 'start' || action === 'reload' || action === 'workspace') {
    state.workspace = workspace || state.workspace;
  }
  if (action === 'start' || action === 'reload' || action === 'session') {
    state.sessionId = sessionId || state.sessionId;
  }
  if (action === 'start') {
    state.startedAt ??= now;
    state.stoppedAt = null;
  }
  if (action === 'reload') {
    state.startedAt ??= now;
    state.stoppedAt = null;
  }
  if (action === 'stop') {
    state.stoppedAt = now;
  }
  if (action === 'workspace') {
    state.workspace = workspace || state.workspace;
  }
  if (action === 'session') {
    state.sessionId = sessionId || state.sessionId;
  }

  state.mode = mode;
  state.updatedAt = now;
  state.lifecycle.push({
    action,
    dataDir: state.dataDir,
    mode: state.mode,
    platformId: state.platformId,
    reason,
    reloadCount: state.reloadCount,
    sessionId: state.sessionId,
    status: state.status,
    timestamp: now,
    workspace: state.workspace,
  });
  persistWebRuntimeState(state);
  return state.lifecycle[state.lifecycle.length - 1];
}

function readWebRuntimeState(): WebRuntimeState {
  const fallback = createDefaultWebRuntimeState();
  if (typeof window === 'undefined') {
    return fallback;
  }
  try {
    const raw = window.localStorage.getItem(WEB_RUNTIME_STATE_KEY);
    if (!raw) {
      return fallback;
    }
    const parsed = JSON.parse(raw) as Partial<WebRuntimeState>;
    return {
      ...fallback,
      ...parsed,
      lifecycle: Array.isArray(parsed.lifecycle)
        ? parsed.lifecycle
        : fallback.lifecycle,
    };
  } catch {
    return fallback;
  }
}

function persistWebRuntimeState(state: WebRuntimeState) {
  if (typeof window === 'undefined') {
    return;
  }
  window.localStorage.setItem(WEB_RUNTIME_STATE_KEY, JSON.stringify(state));
}

function createDefaultWebRuntimeState(): WebRuntimeState {
  const now = Date.now();
  return {
    dataDir: 'browser',
    lifecycle: [],
    mode: 'web',
    platformId: 'web',
    reloadCount: 0,
    schemaVersion: 'app-runtime/v1',
    sessionId: null,
    startedAt: null,
    status: 'initialized',
    stoppedAt: null,
    updatedAt: now,
    workspace: null,
  };
}

function pickTrimmedString(
  value: unknown,
  fallback: null | string,
): null | string {
  if (typeof value === 'string') {
    const trimmed = value.trim();
    return trimmed || fallback;
  }
  return fallback;
}

async function writeWebClipboard(input: Record<string, unknown>) {
  const text = typeof input.text === 'string' ? input.text : '';
  if (!navigator.clipboard?.writeText) {
    throw new Error('当前浏览器不支持 navigator.clipboard.writeText');
  }
  await navigator.clipboard.writeText(text);
  return text.length;
}

async function readWebClipboard() {
  if (!navigator.clipboard?.readText) {
    throw new Error('当前浏览器不支持 navigator.clipboard.readText');
  }
  return await navigator.clipboard.readText();
}

async function sendWebNotification(input: Record<string, unknown>) {
  const title = typeof input.title === 'string' ? input.title : 'AIO';
  const body = typeof input.body === 'string' ? input.body : '';
  if (!('Notification' in window)) {
    throw new Error('当前浏览器不支持 Notification');
  }
  const permission =
    Notification.permission === 'default'
      ? await Notification.requestPermission()
      : Notification.permission;
  if (permission !== 'granted') {
    throw new Error('浏览器通知权限未授权');
  }
  const notification = new Notification(title, { body });
  return notification;
}

export async function callAuthedCommand<T>(
  command: string,
  payload: Record<string, unknown> = {},
) {
  const token = getToken();
  return await callCommand<T>(command, { token, ...payload });
}

export interface PageRequest {
  keyword?: string;
  o?: number;
  s?: number;
}

export interface PageResult<T> {
  d: T[];
  p: {
    o: number;
    s: number;
  };
  t: number;
}
