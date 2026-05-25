export const pluginId = 'git-suite';
export const displayName = 'Git Suite';

export async function activate(context = {}) {
  context.log?.(`${displayName} activated`);
  return {
    extensionPoints: ['git-suite.remoteProvider', 'git-suite.commitPolicy'],
    pluginId,
  };
}

export async function deactivate(context = {}) {
  context.log?.(`${displayName} deactivated`);
}

export async function dispose(context = {}) {
  context.log?.(`${displayName} disposed`);
}
