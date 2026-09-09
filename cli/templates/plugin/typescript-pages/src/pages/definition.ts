type SceneDefinition = {
  id: string;
  label: string;
};

type CounterBody = {
  kind: "counter";
  title: string;
  button: string;
};

type TextBody = {
  kind: "text";
  title: string;
  content: string;
};

type PageDefinition = {
  id: string;
  label: string;
  icon: string | null;
  scene: SceneDefinition;
  required_permission: string | null;
  body: CounterBody | TextBody;
};

export const pages = [
  {
    id: __PRIMARY_ID__,
    label: __TITLE__,
    icon: "braces",
    scene: { id: "community", label: "社区插件" },
    required_permission: null,
    body: {
      kind: "counter",
      title: "TypeScript PageDefinition",
      button: "TypeScript +1",
    },
  },
  {
    id: __ABOUT_ID__,
    label: "TypeScript 说明",
    icon: "info",
    scene: { id: "community", label: "社区插件" },
    required_permission: null,
    body: {
      kind: "text",
      title: "由 TypeScript 构建",
      content: "页面定义在构建时生成，安装后由宿主原生渲染。",
    },
  },
] satisfies PageDefinition[];

export function definitionJson(): string {
  if (pages.length === 0 || new Set(pages.map(({ id }) => id)).size !== pages.length) {
    throw new Error("插件页面不能为空且页面 ID 不能重复");
  }
  return JSON.stringify(pages);
}
