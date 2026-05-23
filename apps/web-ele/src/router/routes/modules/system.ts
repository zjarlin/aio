import type { RouteRecordRaw } from 'vue-router';

import { BasicLayout } from '#/layouts';
import { $t } from '#/locales';

const routes: RouteRecordRaw[] = [
  {
    component: BasicLayout,
    meta: {
      icon: 'lucide:settings',
      order: 100,
      title: $t('page.system.title'),
    },
    name: 'System',
    path: '/system',
    children: [
      {
        name: 'SystemUsers',
        path: '/system/users',
        component: () => import('#/views/system/users/index.vue'),
        meta: {
          icon: 'lucide:users',
          title: $t('page.system.users'),
        },
      },
      {
        name: 'SystemRoles',
        path: '/system/roles',
        component: () => import('#/views/system/roles/index.vue'),
        meta: {
          icon: 'lucide:shield-check',
          title: $t('page.system.roles'),
        },
      },
      {
        name: 'SystemPermissions',
        path: '/system/permissions',
        component: () => import('#/views/system/permissions/index.vue'),
        meta: {
          icon: 'lucide:key-round',
          title: $t('page.system.permissions'),
        },
      },
      {
        name: 'SystemDictionaries',
        path: '/system/dictionaries',
        component: () => import('#/views/system/dictionaries/index.vue'),
        meta: {
          icon: 'lucide:book-type',
          title: $t('page.system.dictionaries'),
        },
      },
    ],
  },
];

export default routes;
