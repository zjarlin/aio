import { callAuthedCommand, callCommand } from '#/api/local/client';

export namespace AuthApi {
  /** 登录接口参数 */
  export interface LoginParams {
    password?: string;
    username?: string;
  }

  /** 登录接口返回值 */
  export interface LoginResult {
    accessToken: string;
  }

  export interface RefreshTokenResult {
    data: string;
    status: number;
  }
}

/**
 * 登录
 */
export async function loginApi(data: AuthApi.LoginParams) {
  return await callCommand<AuthApi.LoginResult>('auth_login', {
    request: data,
  });
}

/**
 * 刷新accessToken
 */
export async function refreshTokenApi() {
  throw new Error('AIO 本地桌面模式不支持刷新 token');
}

/**
 * 退出登录
 */
export async function logoutApi() {
  return await callAuthedCommand<null>('auth_logout');
}

/**
 * 获取用户权限码
 */
export async function getAccessCodesApi() {
  return await callAuthedCommand<string[]>('auth_access_codes');
}
