export const pluginId = 'platform.macos-clipboard';
export const displayName = 'macOS Clipboard Sample';

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
