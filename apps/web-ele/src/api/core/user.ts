import type { UserInfo } from '@vben/types';

import { callAuthedCommand } from '#/api/local/client';

/**
 * 获取用户信息
 */
export async function getUserInfoApi() {
  return await callAuthedCommand<UserInfo>('auth_current_user');
}
