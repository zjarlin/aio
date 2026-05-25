export const pluginId = 'git-suite.github-provider';
export const displayName = 'GitHub Provider';

export async function activate(context = {}) {
  context.log?.(`${displayName} activated`);
  return {
    implements: 'git.remote-provider.v1',
    pluginId,
  };
}

export async function deactivate(context = {}) {
  context.log?.(`${displayName} deactivated`);
}

export async function dispose(context = {}) {
  context.log?.(`${displayName} disposed`);
}
