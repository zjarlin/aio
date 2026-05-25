export async function activate(api: {
  capabilities: {
    invoke<T = unknown>(capability: string, input: unknown): Promise<T>;
  };
  events?: {
    publish(type: string, payload: unknown): Promise<void>;
  };
}) {
  return {
    async appendPermissionDecision(record: unknown) {
      const line = `${JSON.stringify(record)}\n`;
      await api.capabilities.invoke('fs.write', {
        append: true,
        content: line,
        createDirs: true,
        path: 'app-data/plugin-audit/permission-decisions.jsonl',
      });
      await api.events?.publish('permission-core.audit-file-sink.appended', {
        bytes: line.length,
      });
    },
  };
}
