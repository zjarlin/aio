import type { RouteRecordStringComponent } from '@vben/types';

import { callAuthedCommand } from '#/api/local/client';

/**
 * 获取用户所有菜单
 */
export async function getAllMenusApi() {
  return await callAuthedCommand<RouteRecordStringComponent[]>('menu_list');
}
