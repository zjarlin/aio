export const pluginId = 'platform.windows-registry';
export const displayName = 'Windows Registry Sample';

export const windowsRegistryProvider = {
  capability: 'windows.registry.read',
  fallback: 'unsupported',
  kind: 'native',
  platform: 'windows',
  trustLevel: 'trusted-provider',
};

export async function activate(context = {}) {
  context.log?.(`${displayName} activated`);
  return { pluginId };
}

export async function deactivate(context = {}) {
  context.log?.(`${displayName} deactivated`);
}

export async function dispose(context = {}) {
  context.log?.(`${displayName} disposed`);
}
