<script lang="ts" setup>
import { computed, onMounted, reactive, ref } from 'vue';

import { Page } from '@vben/common-ui';

import {
  ElButton,
  ElDialog,
  ElInput,
  ElMessageBox,
  ElOption,
  ElSelect,
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

const notes = ref<NoteRecord[]>([]);
const keyword = ref('');
const showArchived = ref(false);
const editorVisible = ref(false);
const editingId = ref('');
const form = reactive({
  category: '',
  content: '',
  tagsText: '',
  title: '',
});
const title = computed(() => (editingId.value ? '编辑笔记' : '新增笔记'));

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
}

function openCreate() {
  editingId.value = '';
  Object.assign(form, {
    category: '',
    content: '',
    tagsText: '',
    title: '',
  });
  editorVisible.value = true;
}

async function quickCreate(content: string) {
  await noteCreateApi({
    category: '快速记录',
    content,
    tags: parseTags(content),
    title: inferTitle(content),
  });
  await loadNotes();
}

function openEdit(note: NoteRecord) {
  editingId.value = note.id;
  Object.assign(form, {
    category: note.category,
    content: note.content,
    tagsText: note.tags.join(', '),
    title: note.title,
  });
  editorVisible.value = true;
}

async function saveNote() {
  const input = {
    category: form.category,
    content: form.content,
    tags: form.tagsText
      .split(',')
      .map((tag) => tag.trim())
      .filter(Boolean),
    title: form.title,
  };
  await (editingId.value
    ? noteUpdateApi({ id: editingId.value, ...input })
    : noteCreateApi(input));
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
        <ElButton type="primary" @click="openCreate">新增笔记</ElButton>
      </div>

      <div class="grid grid-cols-1 gap-3 xl:grid-cols-2">
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
            <ElTag v-if="note.isFavorite" type="warning">收藏</ElTag>
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
            <ElButton link type="warning" @click="toggleFavorite(note)">
              {{ note.isFavorite ? '取消收藏' : '收藏' }}
            </ElButton>
            <ElButton link type="primary" @click="openEdit(note)">
              编辑
            </ElButton>
            <ElButton link type="info" @click="toggleArchive(note)">
              {{ note.isArchived ? '恢复' : '归档' }}
            </ElButton>
            <ElButton link type="danger" @click="deleteNote(note)">
              删除
            </ElButton>
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
  </Page>
</template>
