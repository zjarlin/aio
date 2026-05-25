import type { PluginUiRenderSchema, PluginUiTreeNode } from '@vben/types';

export function renderPluginViewToText(view: PluginUiRenderSchema) {
  const lines = renderPluginViewLines(view);
  return lines.join('\n');
}

export function renderPluginViewLines(view: PluginUiRenderSchema): string[] {
  const title = view.title || view.kind;
  switch (view.kind) {
    case 'detail':
    case 'summary-list': {
      return [
        `# ${title}`,
        ...view.items.map((item) => `${item.label}: ${item.value}`),
      ];
    }
    case 'form': {
      return [
        `# ${title}`,
        ...view.fields.map((field) => {
          const marker = field.required ? ' *' : '';
          return `${field.label}${marker}: ${field.value}`;
        }),
        ...(view.submitLabel ? [`action: ${view.submitLabel}`] : []),
      ];
    }
    case 'graph': {
      return [
        `# ${title}`,
        'nodes:',
        ...view.nodes.map((node) => {
          const group = node.group ? ` [${node.group}]` : '';
          const value = node.value ? `: ${node.value}` : '';
          return `- ${node.label}${group}${value}`;
        }),
        'edges:',
        ...view.edges.map((edge) => {
          const label = edge.label ? ` --${edge.label}--> ` : ' --> ';
          return `- ${edge.from}${label}${edge.to}`;
        }),
      ];
    }
    case 'markdown': {
      return view.content.split('\n');
    }
    case 'table': {
      const headers = view.columns.map((column) => column.label);
      const keys = view.columns.map((column) => column.key);
      return [
        `# ${title}`,
        headers.join(' | '),
        headers.map(() => '---').join(' | '),
        ...view.rows.map((row) => {
          return keys.map((key) => formatTextValue(row[key])).join(' | ');
        }),
      ];
    }
    case 'timeline': {
      return [
        `# ${title}`,
        ...view.items.map((item) => {
          const time = item.time ? `[${item.time}] ` : '';
          return `- ${time}${item.label}: ${item.value}`;
        }),
      ];
    }
    case 'tree': {
      return [`# ${title}`, ...renderTreeNodes(view.nodes)];
    }
    case 'wizard': {
      return [
        `# ${title}`,
        ...view.steps.map((step) => {
          const marker = step.id === view.activeStep ? '*' : '-';
          const description = step.description ? `: ${step.description}` : '';
          return `${marker} ${step.title}${description}`;
        }),
      ];
    }
  }
}

function renderTreeNodes(nodes: PluginUiTreeNode[], depth = 0): string[] {
  const prefix = '  '.repeat(depth);
  return nodes.flatMap((node) => {
    const value = node.value ? ` (${node.value})` : '';
    return [
      `${prefix}- ${node.label}${value}`,
      ...renderTreeNodes(node.children || [], depth + 1),
    ];
  });
}

function formatTextValue(value: unknown) {
  if (value === null || value === undefined || value === '') {
    return '-';
  }
  if (typeof value === 'string') {
    return value;
  }
  if (typeof value === 'number' || typeof value === 'boolean') {
    return `${value}`;
  }
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}
