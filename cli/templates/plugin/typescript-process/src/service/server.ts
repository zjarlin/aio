import { createServer, type IncomingMessage, type ServerResponse } from "node:http";

const DEFAULT_PORT = 8080;
const MAX_REQUEST_BYTES = 4 * 1024 * 1024;

type RuntimeContext = {
  tenantId: string;
  userId: string;
};

type PageActionRequest = {
  kind?: unknown;
  page_id?: unknown;
  action_id?: unknown;
  tenant_id?: unknown;
  user_id?: unknown;
  body?: {
    kind?: unknown;
    state?: {
      count?: unknown;
    };
  };
};

function page(count = 0) {
  return {
    id: "ts-process",
    label: "TS 服务",
    icon: "server",
    scene: {
      id: "community",
      label: "社区插件",
    },
    required_permission: null,
    body: {
      kind: "actions",
      title: "TypeScript 进程插件 v2 已在线",
      content: `计数：${count}`,
      state: { count },
      actions: [{ id: "increment", label: "TypeScript +1" }],
    },
  } as const;
}

const port = Number.parseInt(process.env.AIO_PLUGIN_PORT ?? String(DEFAULT_PORT), 10);
if (!Number.isSafeInteger(port) || port < 1 || port > 65535) {
  throw new Error("AIO_PLUGIN_PORT 必须是有效端口");
}

const server = createServer(async (request, response) => {
  try {
    const url = new URL(request.url ?? "/", "http://plugin.internal");
    if (request.method === "GET" && url.pathname === "/health") {
      respond(response, 200, "text/plain; charset=utf-8", "ok");
      return;
    }
    if (request.method === "GET" && url.pathname === "/aio/definition") {
      respond(response, 200, "application/json; charset=utf-8", JSON.stringify([page()]));
      return;
    }
    if (request.method === "POST" && url.pathname === "/aio/action") {
      const action = JSON.parse(await readBody(request)) as PageActionRequest;
      if (action.kind !== "page_action") {
        respond(
          response,
          400,
          "application/json; charset=utf-8",
          JSON.stringify({ error: "plugin request kind is not supported" }),
        );
        return;
      }
      if (action.page_id !== "ts-process" || action.action_id !== "increment") {
        respond(
          response,
          400,
          "application/json; charset=utf-8",
          JSON.stringify({ error: "page action is not declared" }),
        );
        return;
      }
      if (
        typeof action.tenant_id !== "string" ||
        action.tenant_id.length === 0 ||
        typeof action.user_id !== "string" ||
        action.user_id.length === 0 ||
        action.body?.kind !== "actions"
      ) {
        respond(
          response,
          400,
          "application/json; charset=utf-8",
          JSON.stringify({ error: "trusted page action context is required" }),
        );
        return;
      }
      const count = action.body?.state?.count;
      if (!Number.isSafeInteger(count) || Number(count) < 0) {
        respond(
          response,
          400,
          "application/json; charset=utf-8",
          JSON.stringify({ error: "page state count must be a non-negative integer" }),
        );
        return;
      }
      respond(
        response,
        200,
        "application/json; charset=utf-8",
        JSON.stringify({ body: page(Number(count) + 1).body }),
      );
      return;
    }
    if (url.pathname === "/echo") {
      const body = await readBody(request);
      const context = readContext(request);
      respond(
        response,
        200,
        "application/json; charset=utf-8",
        JSON.stringify({
          runtime: "node-typescript",
          version: 2,
          method: request.method ?? "GET",
          path: url.pathname,
          query: url.search === "" ? null : url.search.slice(1),
          body,
          tenant_id: context.tenantId,
          user_id: context.userId,
        }),
      );
      return;
    }
    respond(response, 404, "application/json; charset=utf-8", JSON.stringify({ error: "not found" }));
  } catch (error) {
    const message = error instanceof Error ? error.message : "internal error";
    respond(response, 500, "application/json; charset=utf-8", JSON.stringify({ error: message }));
  }
});

process.on("SIGTERM", () => server.close(() => undefined));
process.on("SIGINT", () => server.close(() => undefined));
server.listen(port, "0.0.0.0", () => console.log(`AIO TypeScript process plugin listening on ${port}`));

function readContext(request: IncomingMessage): RuntimeContext {
  return {
    tenantId: header(request, "x-aio-tenant-id"),
    userId: header(request, "x-aio-user-id"),
  };
}

function header(request: IncomingMessage, name: string): string {
  const value = request.headers[name];
  return Array.isArray(value) ? (value[0] ?? "") : (value ?? "");
}

async function readBody(request: IncomingMessage): Promise<string> {
  const chunks: Uint8Array[] = [];
  let size = 0;
  return new Promise((resolve, reject) => {
    request.on("data", (chunk) => {
      size += chunk.byteLength;
      if (size > MAX_REQUEST_BYTES) {
        request.destroy();
        reject(new Error("request body exceeds 4 MiB"));
        return;
      }
      chunks.push(chunk);
    });
    request.on("end", () => resolve(new TextDecoder().decode(concat(chunks, size))));
    request.on("error", reject);
  });
}

function concat(chunks: Uint8Array[], size: number): Uint8Array {
  const result = new Uint8Array(size);
  let offset = 0;
  for (const chunk of chunks) {
    result.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return result;
}

function respond(response: ServerResponse, status: number, contentType: string, body: string): void {
  response.setHeader("Content-Type", contentType);
  response.writeHead(status);
  response.end(body);
}
