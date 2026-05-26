export const pluginId = 'platform.macos-automation';
export const displayName = 'macOS Automation Sample';

export const macosAutomationProvider = {
  capability: 'macos.automation',
  fallback: 'approval-required',
  kind: 'native',
  platform: 'macos',
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
