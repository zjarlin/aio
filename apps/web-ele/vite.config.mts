import { defineConfig } from '@vben/vite-config';

import ElementPlus from 'unplugin-element-plus/vite';

export default defineConfig(async () => {
  return {
    application: {},
    vite: {
      clearScreen: false,
      plugins: [
        ElementPlus({
          format: 'esm',
        }),
      ],
      server: {
        watch: {
          // 告诉 Vite 忽略监听 `src-tauri` 目录
          ignored: ['**/src-tauri/**'],
        },
      },
    },
  };
});
