<script lang="ts" setup>
import type { PluginUiRenderSchema, PluginUiSchemaKind } from '@vben/types';

import { computed } from 'vue';

import { renderPluginViewToText } from '@vben/plugins/view-renderer';

import {
  ElDescriptions,
  ElDescriptionsItem,
  ElTable,
  ElTableColumn,
  ElTag,
  ElTimeline,
  ElTimelineItem,
  ElTree,
} from 'element-plus';

const props = defineProps<{
  view: PluginUiRenderSchema;
}>();

const schemaLabels: Record<PluginUiSchemaKind, string> = {
  detail: '详情',
  form: '表单',
  graph: '关系图',
  markdown: 'Markdown',
  'summary-list': '摘要',
  table: '表格',
  timeline: '时间线',
  tree: '树',
  wizard: '向导',
};

const title = computed(() => props.view.title || schemaLabels[props.view.kind]);
const textPreview = computed(() => renderPluginViewToText(props.view));

function formatValue(value: unknown) {
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

function cellValue(row: Record<string, unknown>, key: string) {
  return formatValue(row[key]);
}

function graphNodeStyle(index: number, total: number) {
  const angle = total <= 1 ? 0 : (Math.PI * 2 * index) / total - Math.PI / 2;
  const radius = 38;
  const x = 50 + Math.cos(angle) * radius;
  const y = 50 + Math.sin(angle) * radius;
  return {
    left: `${x}%`,
    top: `${y}%`,
  };
}
</script>

<template>
  <div class="border-border/60 space-y-3 rounded-sm border p-3">
    <div class="flex items-center justify-between gap-3">
      <div class="min-w-0">
        <div class="truncate text-sm font-medium">
          {{ title }}
        </div>
        <div class="text-muted-foreground text-xs">
          {{ schemaLabels[view.kind] }}
        </div>
      </div>
      <ElTag size="small" type="info">{{ view.kind }}</ElTag>
    </div>

    <template v-if="view.kind === 'summary-list' || view.kind === 'detail'">
      <ElDescriptions :column="2" border size="small">
        <ElDescriptionsItem
          v-for="item in view.items"
          :key="item.label"
          :label="item.label"
        >
          <span class="break-all">{{ item.value }}</span>
          <span v-if="item.hint" class="text-muted-foreground ml-2 text-xs">
            {{ item.hint }}
          </span>
        </ElDescriptionsItem>
      </ElDescriptions>
    </template>

    <template v-else-if="view.kind === 'table'">
      <ElTable :data="view.rows" border size="small" stripe>
        <ElTableColumn
          v-for="column in view.columns"
          :key="column.key"
          :align="column.align"
          :label="column.label"
          :prop="column.key"
          :width="column.width"
          min-width="120"
        >
          <template #default="{ row }">
            <span class="break-all">{{ cellValue(row, column.key) }}</span>
          </template>
        </ElTableColumn>
      </ElTable>
    </template>

    <template v-else-if="view.kind === 'tree'">
      <ElTree
        :data="view.nodes"
        :props="{ children: 'children', label: 'label' }"
        default-expand-all
        node-key="id"
      >
        <template #default="{ data }">
          <div class="flex items-center gap-2">
            <span class="font-medium">{{ data.label }}</span>
            <ElTag v-if="data.value" size="small" type="info">
              {{ data.value }}
            </ElTag>
          </div>
        </template>
      </ElTree>
    </template>

    <template v-else-if="view.kind === 'graph'">
      <div class="grid gap-3 lg:grid-cols-[minmax(0,1fr)_minmax(260px,360px)]">
        <div
          class="border-border/60 bg-muted/20 relative h-64 overflow-hidden rounded-sm border"
        >
          <div
            v-for="(edge, index) in view.edges"
            :key="`${edge.from}-${edge.to}-${index}`"
            :style="{ transform: `translateY(${index * 30}px)` }"
            class="border-border/60 bg-background/85 absolute left-4 top-4 max-w-[calc(100%-2rem)] truncate rounded-sm border px-2 py-1 text-xs shadow-sm"
          >
            {{ edge.from }}
            <span class="text-muted-foreground">
              {{ edge.label ? ` -> ${edge.label} -> ` : ' -> ' }}
            </span>
            {{ edge.to }}
          </div>
          <div
            v-for="(node, index) in view.nodes"
            :key="node.id"
            :style="graphNodeStyle(index, view.nodes.length)"
            class="border-primary/30 bg-background absolute min-w-24 max-w-36 -translate-x-1/2 -translate-y-1/2 rounded-sm border px-2 py-1 text-center text-xs shadow-sm"
          >
            <div class="truncate font-medium">{{ node.label }}</div>
            <div
              v-if="node.group || node.value"
              class="text-muted-foreground truncate"
            >
              {{ node.group || node.value }}
            </div>
          </div>
        </div>
        <div class="space-y-2">
          <div class="text-muted-foreground text-xs">节点</div>
          <div class="flex flex-wrap gap-2">
            <ElTag v-for="node in view.nodes" :key="node.id" size="small">
              {{ node.label }}
            </ElTag>
          </div>
          <div class="text-muted-foreground text-xs">关系</div>
          <div class="space-y-1 text-xs">
            <div
              v-for="(edge, index) in view.edges"
              :key="`${edge.from}-${edge.to}-${index}-list`"
              class="bg-muted/30 break-all rounded-sm px-2 py-1"
            >
              {{ edge.from }}
              <span class="text-muted-foreground">
                {{ edge.label ? ` -> ${edge.label} -> ` : ' -> ' }}
              </span>
              {{ edge.to }}
            </div>
          </div>
        </div>
      </div>
    </template>

    <template v-else-if="view.kind === 'form'">
      <ElDescriptions :column="2" border size="small">
        <ElDescriptionsItem
          v-for="field in view.fields"
          :key="field.label"
          :label="field.label"
        >
          <div class="space-y-1">
            <div class="break-all">{{ field.value }}</div>
            <div v-if="field.hint" class="text-muted-foreground text-xs">
              {{ field.hint }}
            </div>
            <ElTag v-if="field.required" size="small" type="warning">
              必填
            </ElTag>
          </div>
        </ElDescriptionsItem>
      </ElDescriptions>
      <div v-if="view.submitLabel" class="text-muted-foreground text-xs">
        提交动作: {{ view.submitLabel }}
      </div>
    </template>

    <template v-else-if="view.kind === 'timeline'">
      <ElTimeline>
        <ElTimelineItem
          v-for="item in view.items"
          :key="`${item.label}-${item.time || item.value}`"
          :timestamp="item.time"
          :type="item.tone || 'info'"
        >
          <div class="space-y-1">
            <div class="font-medium">{{ item.label }}</div>
            <div class="break-all text-sm">{{ item.value }}</div>
          </div>
        </ElTimelineItem>
      </ElTimeline>
    </template>

    <template v-else-if="view.kind === 'wizard'">
      <ElTimeline>
        <ElTimelineItem
          v-for="step in view.steps"
          :key="step.id"
          :type="step.id === view.activeStep ? 'primary' : 'info'"
        >
          <div class="space-y-1">
            <div class="font-medium">{{ step.title }}</div>
            <div v-if="step.description" class="break-all text-sm">
              {{ step.description }}
            </div>
          </div>
        </ElTimelineItem>
      </ElTimeline>
    </template>

    <template v-else>
      <pre
        class="bg-muted/40 whitespace-pre-wrap break-words rounded-sm p-3 text-xs leading-6"
        v-text="textPreview"
      ></pre>
    </template>
  </div>
</template>
