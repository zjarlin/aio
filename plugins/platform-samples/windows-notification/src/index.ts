export const pluginId = 'platform.windows-notification';
export const displayName = 'Windows Notification Sample';
export const windowsNotificationProvider = {
  capability: 'notification.send',
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
