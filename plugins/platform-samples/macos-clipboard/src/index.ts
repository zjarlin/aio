export const pluginId = 'platform.macos-clipboard';
export const displayName = 'macOS Clipboard Sample';
export const macosClipboardProvider = {
  capability: 'clipboard.write',
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
