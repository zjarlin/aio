import { useAccessStore } from '@vben/stores';

import { invoke } from '@tauri-apps/api/core';
import { ElMessage } from 'element-plus';

interface CommandError {
  code: number;
  details?: string;
  message: string;
}

const BRIDGE_RETRY_DELAYS_MS = [150, 350, 800];

function getToken() {
  const accessStore = useAccessStore();
  return accessStore.accessToken ?? '';
}

const commandBridgeUrl =
  import.meta.env.VITE_AIO_COMMAND_BRIDGE_URL ||
  (import.meta.env.DEV ? 'http://127.0.0.1:18777/__aio/command' : '');

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

function isRetryableBridgeError(error: unknown) {
  if (error instanceof TypeError) {
    return true;
  }
  if (error instanceof Error) {
    return /Failed to fetch|NetworkError|Load failed/i.test(error.message);
  }
  return false;
}

function sleep(ms: number) {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
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
      let lastError: unknown;
      for (const delay of [0, ...BRIDGE_RETRY_DELAYS_MS]) {
        if (delay > 0) {
          await sleep(delay);
        }
        try {
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
        } catch (error) {
          lastError = error;
          if (!isRetryableBridgeError(error)) {
            throw error;
          }
        }
      }
      throw lastError;
    }
    throw new Error('当前浏览器环境未启用 AIO 本地命令桥，请使用桌面端');
  } catch (error) {
    const commandError = normalizeError(error);
    ElMessage.error(commandError.message);
    throw commandError;
  }
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
