import type { RouteRecordRaw } from 'vue-router';

import { BasicLayout } from '#/layouts';
import { $t } from '#/locales';

const routes: RouteRecordRaw[] = [
  {
    component: BasicLayout,
    meta: {
      icon: 'lucide:archive',
      order: 200,
      title: $t('page.assets.title'),
    },
    name: 'Assets',
    path: '/assets',
    redirect: '/assets/notes',
    children: [
      {
        name: 'AssetNotes',
        path: '/assets/notes',
        component: () => import('#/views/assets/notes/index.vue'),
        meta: {
          icon: 'lucide:sticky-note',
          title: $t('page.assets.notes'),
        },
      },
      {
        name: 'AssetSkills',
        path: '/assets/skills',
        component: () => import('#/views/assets/skills/index.vue'),
        meta: {
          icon: 'lucide:sparkles',
          title: $t('page.assets.skills'),
        },
      },
      {
        name: 'AssetAgentPreferences',
        path: '/assets/agent-preferences',
        component: () => import('#/views/assets/agent-preferences/index.vue'),
        meta: {
          icon: 'lucide:bot',
          title: $t('page.assets.agentPreferences'),
        },
      },
      {
        name: 'AssetOpenAIAssistant',
        path: '/assets/openai-assistant',
        component: () => import('#/views/assets/openai-assistant/index.vue'),
        meta: {
          icon: 'lucide:bot',
          title: $t('page.assets.openaiAssistant'),
        },
      },
      {
        name: 'AssetDockerCompose',
        path: '/assets/docker-compose',
        component: () => import('#/views/assets/docker-compose/index.vue'),
        meta: {
          icon: 'lucide:container',
          title: $t('page.assets.dockerCompose'),
        },
      },
      {
        name: 'AssetCli',
        path: '/assets/cli',
        component: () => import('#/views/assets/cli/index.vue'),
        meta: {
          icon: 'lucide:terminal',
          title: $t('page.assets.cli'),
        },
      },
      {
        name: 'AssetEnvVars',
        path: '/assets/env-vars',
        component: () => import('#/views/assets/env-vars/index.vue'),
        meta: {
          icon: 'lucide:variable',
          title: $t('page.assets.envVars'),
        },
      },
      {
        name: 'AssetBashFunctions',
        path: '/assets/bash-functions',
        component: () => import('#/views/assets/bash-functions/index.vue'),
        meta: {
          icon: 'lucide:square-function',
          title: $t('page.assets.bashFunctions'),
        },
      },
      {
        name: 'AssetDotfiles',
        path: '/assets/dotfiles',
        component: () => import('#/views/assets/dotfiles/index.vue'),
        meta: {
          icon: 'lucide:file-cog',
          title: $t('page.assets.dotfiles'),
        },
      },
    ],
  },
];

export default routes;
