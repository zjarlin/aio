import type { PluginUiRenderSchema } from '@vben/types';

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import process from 'node:process';

import { describe, expect, it } from 'vitest';

import { renderPluginViewLines, renderPluginViewToText } from './index';

describe('renderPluginViewToText', () => {
  it('renders graph contract text consistently', () => {
    const view: PluginUiRenderSchema = {
      edges: [
        { from: 'registry', label: 'loads', to: 'host' },
        { from: 'host', label: 'requests', to: 'broker' },
      ],
      kind: 'graph',
      nodes: [
        { id: 'registry', label: 'Registry', group: 'core' },
        { id: 'host', label: 'Extension Host', group: 'runtime' },
        { id: 'broker', label: 'Capability Broker', value: 'audited' },
      ],
      title: '插件关系图',
    };

    expect(renderPluginViewLines(view)).toEqual([
      '# 插件关系图',
      'nodes:',
      '- Registry [core]',
      '- Extension Host [runtime]',
      '- Capability Broker: audited',
      'edges:',
      '- registry --loads--> host',
      '- host --requests--> broker',
    ]);
  });

  it('renders wizard contract text consistently', () => {
    const view: PluginUiRenderSchema = {
      activeStep: 'validate',
      kind: 'wizard',
      steps: [
        {
          id: 'formula',
          title: '公式',
          description: '维护 formula.json 作为事实来源',
        },
        {
          id: 'generate',
          title: '生成',
          description: '生成胶囊源码和 smoke test',
        },
        {
          id: 'validate',
          title: '校验',
          description: '运行注册表和类型检查',
        },
        {
          id: 'publish',
          title: '发布',
          description: '写入本地 registry 并保留审计',
        },
      ],
      title: '插件自举向导',
    };

    expect(renderPluginViewLines(view)).toEqual([
      '# 插件自举向导',
      '- 公式: 维护 formula.json 作为事实来源',
      '- 生成: 生成胶囊源码和 smoke test',
      '* 校验: 运行注册表和类型检查',
      '- 发布: 写入本地 registry 并保留审计',
    ]);
    expect(renderPluginViewToText(view)).toBe(
      [
        '# 插件自举向导',
        '- 公式: 维护 formula.json 作为事实来源',
        '- 生成: 生成胶囊源码和 smoke test',
        '* 校验: 运行注册表和类型检查',
        '- 发布: 写入本地 registry 并保留审计',
      ].join('\n'),
    );
  });
});

describe('plugin-ui-view schema', () => {
  it('keeps graph and wizard in the declared contract', () => {
    const schema = JSON.parse(
      readFileSync(
        resolve(process.cwd(), 'schemas/plugin-ui-view.v1.schema.json'),
        'utf8',
      ),
    ) as {
      $defs: {
        graph: {
          properties: {
            edges: { items: { $ref: string } };
            kind: { const: string };
            nodes: { minItems: number };
          };
          required: string[];
        };
        wizard: {
          properties: {
            kind: { const: string };
            steps: { minItems: number };
          };
          required: string[];
        };
      };
      oneOf: Array<{ $ref?: string }>;
    };

    expect(schema.$defs.graph.properties.kind.const).toBe('graph');
    expect(schema.$defs.graph.required).toEqual(['kind', 'nodes', 'edges']);
    expect(schema.$defs.graph.properties.nodes.minItems).toBe(1);
    expect(schema.$defs.graph.properties.edges.items.$ref).toBe(
      '#/$defs/graphEdge',
    );
    expect(schema.oneOf).toContainEqual({ $ref: '#/$defs/graph' });
    expect(schema.$defs.wizard.properties.kind.const).toBe('wizard');
    expect(schema.$defs.wizard.required).toContain('steps');
    expect(schema.$defs.wizard.properties.steps.minItems).toBe(1);
    expect(schema.oneOf).toContainEqual({ $ref: '#/$defs/wizard' });
  });
});
