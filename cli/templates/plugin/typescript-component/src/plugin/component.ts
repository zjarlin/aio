type RuntimeRequest = {
  kind: "service_request";
  method: string;
  path: string;
  query: string | null;
  body: string;
  tenant_id: string;
  user_id: string;
};

type PageActionRequest = {
  kind: "page_action";
  page_id: string;
  action_id: string;
  tenant_id: string;
  user_id: string;
  body: {
    state?: {
      count?: unknown;
    };
  };
};

type ComponentResponse = {
  status: number;
  content_type: string;
  body: string;
};

function page(count = 0) {
  return {
    id: "ts-counter",
    label: "TypeScript",
    icon: "braces",
    scene: {
      id: "community",
      label: "社区插件",
    },
    required_permission: null,
    body: {
      kind: "actions",
      title: "TypeScript Component",
      content: `计数：${count}`,
      state: { count },
      actions: [{ id: "increment", label: "TypeScript +1" }],
    },
  } as const;
}

export function definition(): string {
  return JSON.stringify([page()]);
}

export function handle(request: string): string {
  const parsed = JSON.parse(request) as RuntimeRequest | PageActionRequest;
  if (parsed.kind === "page_action") {
    if (parsed.page_id !== "ts-counter" || parsed.action_id !== "increment") {
      return JSON.stringify({
        status: 400,
        content_type: "application/json",
        body: JSON.stringify({ error: "page action is not declared" }),
      } satisfies ComponentResponse);
    }
    const count = parsed.body.state?.count;
    if (!Number.isSafeInteger(count) || Number(count) < 0) {
      return JSON.stringify({
        status: 400,
        content_type: "application/json",
        body: JSON.stringify({ error: "page state count must be a non-negative integer" }),
      } satisfies ComponentResponse);
    }
    return JSON.stringify({
      status: 200,
      content_type: "application/json",
      body: JSON.stringify({ body: page(Number(count) + 1).body }),
    } satisfies ComponentResponse);
  }
  if (parsed.kind !== "service_request") {
    return JSON.stringify({
      status: 400,
      content_type: "application/json",
      body: JSON.stringify({ error: "plugin request kind is not supported" }),
    } satisfies ComponentResponse);
  }
  const response: ComponentResponse = {
    status: 200,
    content_type: "application/json",
    body: JSON.stringify({
      message: "Hello from TypeScript Component",
      request: parsed,
    }),
  };
  return JSON.stringify(response);
}
