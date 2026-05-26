<script lang="ts" setup>
import { computed, nextTick, onMounted, reactive, ref, watch } from 'vue';

import { Page } from '@vben/common-ui';
import {
  EchartsUI,
  type EchartsUIType,
  getInstanceByDom,
  useEcharts,
} from '@vben/plugins/echarts';
import { useUserStore } from '@vben/stores';

import {
  ElButton,
  ElDialog,
  ElInput,
  ElMessage,
  ElMessageBox,
  ElOption,
  ElSegmented,
  ElSelect,
  ElSwitch,
  ElTag,
} from 'element-plus';

import {
  noteArchiveApi,
  noteCreateApi,
  noteDeleteApi,
  noteFavoriteApi,
  notePageApi,
  type NoteRecord,
  noteUpdateApi,
} from '#/api';

import { formatTime } from '../../system/shared';
import QuickComposer from '../components/quick-composer.vue';

const userStore = useUserStore();
const notes = ref<NoteRecord[]>([]);
const keyword = ref('');
const showArchived = ref(false);
const viewMode = ref<'graph' | 'list'>('graph');
const editorVisible = ref(false);
const editingId = ref('');
const previewVisible = ref(false);
const previewNote = ref<NoteRecord | null>(null);
const chartRef = ref<EchartsUIType>();
const form = reactive({
  category: '',
  content: '',
  isPublic: false,
  tagsText: '',
  title: '',
});
const title = computed(() => (editingId.value ? '编辑笔记' : '新增笔记'));
const currentUserId = computed(() => userStore.userInfo?.userId ?? '');
const { renderEcharts } = useEcharts(chartRef);
type RenderEchartsOption = Parameters<typeof renderEcharts>[0];
const viewOptions = [
  { label: '知识图谱', value: 'graph' },
  { label: '列表', value: 'list' },
];

function parseTags(content: string) {
  const matches = content.match(/#[\u4E00-\u9FA5\w-]+/g) ?? [];
  return [...new Set(matches.map((tag) => tag.slice(1)))];
}

function inferTitle(content: string) {
  const firstLine = content
    .split('\n')
    .map((line) => line.trim())
    .find(Boolean);
  return firstLine?.replace(/^#+\s*/, '').slice(0, 48) || '未命名笔记';
}

async function loadNotes() {
  const result = await notePageApi({
    archived: showArchived.value,
    keyword: keyword.value,
    o: 0,
    s: 100,
  });
  notes.value = result.d;
  await renderGraph();
}

function explainSaveResult(saved: NoteRecord, expectedTitle: string) {
  const reused = saved.title !== expectedTitle;
  ElMessage.success(reused ? '内容已存在，已定位到原笔记' : '笔记已保存');
}

async function quickCreate(content: string, done: (success?: boolean) => void) {
  const title = inferTitle(content);
  try {
    const saved = await noteCreateApi({
      category: '快速记录',
      content,
      tags: parseTags(content),
      title,
    });
    explainSaveResult(saved, title);
    await loadNotes();
    done(true);
  } catch {
    done(false);
  }
}

function openEdit(note: NoteRecord) {
  editingId.value = note.id;
  Object.assign(form, {
    category: note.category,
    content: note.content,
    isPublic: note.isPublic,
    tagsText: note.tags.join(', '),
    title: note.title,
  });
  editorVisible.value = true;
}

function openPreview(note: NoteRecord) {
  previewNote.value = note;
  previewVisible.value = true;
}

function canManage(note: NoteRecord) {
  return note.ownerId === currentUserId.value;
}

function noteSummary(note: NoteRecord) {
  const normalized = note.content.replaceAll(/\s+/g, ' ').trim();
  return normalized.slice(0, 120) || '暂无内容';
}

async function renderGraph() {
  await nextTick();

  const categoryNodes = new Map<string, any>();
  const tagNodes = new Map<string, any>();
  const nodes: any[] = [];
  const links: any[] = [];

  for (const note of notes.value) {
    const noteId = `note:${note.id}`;
    nodes.push({
      category: 0,
      draggable: true,
      id: noteId,
      itemStyle: {
        color: canManage(note) ? '#1d4ed8' : '#059669',
      },
      name: note.title,
      noteId: note.id,
      noteType: 'note',
      symbolSize: Math.max(42, Math.min(72, 34 + note.tags.length * 4)),
      value: noteSummary(note),
    });

    const categoryName = note.category.trim() || '未分类';
    const categoryId = `category:${categoryName}`;
    if (!categoryNodes.has(categoryId)) {
      const categoryNode = {
        category: 1,
        id: categoryId,
        itemStyle: { color: '#f59e0b' },
        name: categoryName,
        noteType: 'category',
        symbolSize: 34,
      };
      categoryNodes.set(categoryId, categoryNode);
      nodes.push(categoryNode);
    }
    links.push({
      lineStyle: { color: '#94a3b8', width: 1.5 },
      source: noteId,
      target: categoryId,
    });

    for (const tag of note.tags) {
      const normalizedTag = tag.trim();
      if (!normalizedTag) {
        continue;
      }
      const tagId = `tag:${normalizedTag}`;
      if (!tagNodes.has(tagId)) {
        const tagNode = {
          category: 2,
          id: tagId,
          itemStyle: { color: '#7c3aed' },
          name: `#${normalizedTag}`,
          noteType: 'tag',
          symbolSize: 26,
        };
        tagNodes.set(tagId, tagNode);
        nodes.push(tagNode);
      }
      links.push({
        lineStyle: { color: '#cbd5e1', width: 1 },
        source: noteId,
        target: tagId,
      });
    }
  }

  const option: RenderEchartsOption = {
    animationDuration: 400,
    legend: {
      data: ['笔记', '分类', '标签'],
      left: 0,
      top: 0,
    },
    series: [
      {
        categories: [{ name: '笔记' }, { name: '分类' }, { name: '标签' }],
        data: nodes,
        edgeSymbol: ['none', 'none'],
        emphasis: {
          focus: 'series',
          label: {
            show: true,
          },
        },
        force: {
          edgeLength: [70, 150],
          gravity: 0.08,
          repulsion: 320,
        },
        label: {
          color: '#334155',
          formatter: '{b}',
          position: 'right',
          show: true,
        },
        layout: 'force',
        lineStyle: {
          opacity: 0.75,
        },
        links,
        roam: true,
        draggable: true,
        symbolKeepAspect: true,
        tooltip: {
          formatter: (params: any) => {
            const data = params.data ?? {};
            if (data.noteType === 'note') {
              return `${data.name}<br/>${data.value ?? ''}`;
            }
            return data.name ?? '';
          },
        },
        type: 'graph',
      } as any,
    ],
    tooltip: {},
  };

  await renderEcharts(option);

  const chart = chartRef.value?.$el
    ? getInstanceByDom(chartRef.value.$el)
    : null;
  chart?.off('click');
  chart?.on('click', (params: any) => {
    const noteId = params?.data?.noteId;
    if (!noteId) {
      return;
    }
    const note = notes.value.find((item) => item.id === noteId);
    if (!note) {
      return;
    }
    if (canManage(note)) {
      openEdit(note);
      return;
    }
    openPreview(note);
  });
}

async function saveNote() {
  const input = {
    category: form.category,
    content: form.content,
    isPublic: form.isPublic,
    tags: form.tagsText
      .split(',')
      .map((tag) => tag.trim())
      .filter(Boolean),
    title: form.title,
  };
  const saved = await (editingId.value
    ? noteUpdateApi({ id: editingId.value, ...input })
    : noteCreateApi(input));
  explainSaveResult(saved, input.title);
  editorVisible.value = false;
  await loadNotes();
}

async function toggleFavorite(note: NoteRecord) {
  await noteFavoriteApi(note.id, !note.isFavorite);
  await loadNotes();
}

async function toggleArchive(note: NoteRecord) {
  await noteArchiveApi(note.id, !note.isArchived);
  await loadNotes();
}

async function deleteNote(note: NoteRecord) {
  await ElMessageBox.confirm(`确认删除笔记 ${note.title}？`, '删除确认');
  await noteDeleteApi(note.id);
  await loadNotes();
}

onMounted(loadNotes);

watch(viewMode, async (mode) => {
  if (mode === 'graph') {
    await renderGraph();
  }
});
</script>

<template>
  <Page description="本地个人 Markdown 笔记" title="笔记">
    <div class="space-y-4">
      <QuickComposer
        placeholder="写下笔记内容，首行会作为标题；支持 Markdown 和 #标签"
        submit-text="保存"
        @submit="quickCreate"
      />

      <div class="flex items-center justify-between gap-3">
        <div class="flex items-center gap-2">
          <ElInput
            v-model="keyword"
            class="w-72"
            clearable
            placeholder="搜索标题或内容"
            @keyup.enter="loadNotes"
          />
          <ElSelect v-model="showArchived" class="w-32" @change="loadNotes">
            <ElOption :value="false" label="活跃笔记" />
            <ElOption :value="true" label="归档笔记" />
          </ElSelect>
          <ElButton @click="loadNotes">查询</ElButton>
        </div>
        <ElSegmented v-model="viewMode" :options="viewOptions" />
      </div>

      <section
        v-if="viewMode === 'graph'"
        class="border-border bg-background rounded border p-3 shadow-sm"
      >
        <EchartsUI ref="chartRef" height="720px" />
      </section>

      <div v-else class="grid grid-cols-1 gap-3 xl:grid-cols-2">
        <article
          v-for="note in notes"
          :key="note.id"
          class="border-border bg-background rounded border p-4 shadow-sm"
        >
          <div class="flex items-start justify-between gap-3">
            <div>
              <h3 class="text-base font-semibold">{{ note.title }}</h3>
              <div class="text-muted-foreground mt-1 text-xs">
                {{ note.category || '未分类' }} ·
                {{ formatTime(note.updatedAt) }}
              </div>
            </div>
            <div class="flex items-center gap-2">
              <ElTag v-if="note.isPublic" type="success">公开</ElTag>
              <ElTag v-if="note.isFavorite" type="warning">收藏</ElTag>
            </div>
          </div>
          <p
            class="text-muted-foreground mt-3 line-clamp-3 whitespace-pre-wrap text-sm"
          >
            {{ note.content || '暂无内容' }}
          </p>
          <div class="mt-3 flex flex-wrap gap-1">
            <ElTag v-for="tag in note.tags" :key="tag" size="small">
              {{ tag }}
            </ElTag>
          </div>
          <div class="mt-4 flex justify-end gap-2">
            <ElButton
              v-if="canManage(note)"
              link
              type="warning"
              @click="toggleFavorite(note)"
            >
              {{ note.isFavorite ? '取消收藏' : '收藏' }}
            </ElButton>
            <ElButton
              v-if="canManage(note)"
              link
              type="primary"
              @click="openEdit(note)"
            >
              编辑
            </ElButton>
            <ElButton
              v-if="canManage(note)"
              link
              type="info"
              @click="toggleArchive(note)"
            >
              {{ note.isArchived ? '恢复' : '归档' }}
            </ElButton>
            <ElButton
              v-if="canManage(note)"
              link
              type="danger"
              @click="deleteNote(note)"
            >
              删除
            </ElButton>
            <span v-if="!canManage(note)" class="text-muted-foreground text-xs">
              公开笔记，仅可查看
            </span>
          </div>
        </article>
      </div>
    </div>

    <ElDialog v-model="editorVisible" :title="title" width="860px">
      <div class="grid grid-cols-2 gap-4">
        <ElInput v-model="form.title" placeholder="标题" />
        <ElInput v-model="form.category" placeholder="分类" />
        <ElInput
          v-model="form.tagsText"
          class="col-span-2"
          placeholder="标签，逗号分隔"
        />
        <div
          class="col-span-2 flex items-center justify-between rounded border px-3 py-2"
        >
          <div>
            <div class="text-sm font-medium">公开给其他用户</div>
            <div class="text-muted-foreground text-xs">
              默认私有。开启后，其他登录用户可以查看这条笔记。
            </div>
          </div>
          <ElSwitch v-model="form.isPublic" />
        </div>
        <ElInput
          v-model="form.content"
          :rows="14"
          class="col-span-2"
          placeholder="Markdown 内容"
          type="textarea"
        />
      </div>
      <template #footer>
        <ElButton @click="editorVisible = false">取消</ElButton>
        <ElButton type="primary" @click="saveNote">保存</ElButton>
      </template>
    </ElDialog>

    <ElDialog v-model="previewVisible" title="公开笔记" width="760px">
      <template v-if="previewNote">
        <div class="space-y-3">
          <div class="flex items-start justify-between gap-3">
            <div>
              <h3 class="text-lg font-semibold">{{ previewNote.title }}</h3>
              <div class="text-muted-foreground mt-1 text-xs">
                {{ previewNote.category || '未分类' }} ·
                {{ formatTime(previewNote.updatedAt) }}
              </div>
            </div>
            <ElTag v-if="previewNote.isPublic" type="success">公开</ElTag>
          </div>
          <div class="flex flex-wrap gap-1">
            <ElTag v-for="tag in previewNote.tags" :key="tag" size="small">
              {{ tag }}
            </ElTag>
          </div>
          <div
            class="bg-muted/30 whitespace-pre-wrap rounded border p-3 text-sm leading-6"
          >
            {{ previewNote.content || '暂无内容' }}
          </div>
        </div>
      </template>
    </ElDialog>
  </Page>
</template>
