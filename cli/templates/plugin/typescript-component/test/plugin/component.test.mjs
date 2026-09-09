import assert from "node:assert/strict";
import test from "node:test";

import { definition, handle } from "../../build/plugin/component.js";

test("definition exposes the TypeScript counter", () => {
  const pages = JSON.parse(definition());
  assert.equal(pages.length, 1);
  assert.equal(pages[0].id, "ts-counter");
  assert.equal(pages[0].body.kind, "actions");
  assert.equal(pages[0].body.content, "计数：0");
  assert.deepEqual(pages[0].body.state, { count: 0 });
  assert.equal(pages[0].body.actions[0].id, "increment");
});

test("handle preserves tenant and user context", () => {
  const request = {
    kind: "service_request",
    method: "POST",
    path: "/echo",
    query: null,
    body: "hello",
    tenant_id: "tenant-a",
    user_id: "user-a",
  };
  const response = JSON.parse(handle(JSON.stringify(request)));
  const body = JSON.parse(response.body);

  assert.equal(response.status, 200);
  assert.equal(response.content_type, "application/json");
  assert.deepEqual(body.request, request);
});

test("handle reduces the host-provided page state", () => {
  const response = JSON.parse(
    handle(
      JSON.stringify({
        kind: "page_action",
        page_id: "ts-counter",
        action_id: "increment",
        tenant_id: "tenant-a",
        user_id: "user-a",
        body: {
          kind: "actions",
          title: "TypeScript Component",
          content: "计数：7",
          state: { count: 7 },
          actions: [{ id: "increment", label: "TypeScript +1" }],
        },
      }),
    ),
  );
  const result = JSON.parse(response.body);

  assert.equal(response.status, 200);
  assert.equal(result.body.kind, "actions");
  assert.equal(result.body.content, "计数：8");
  assert.deepEqual(result.body.state, { count: 8 });
  assert.deepEqual(result.body.actions, [{ id: "increment", label: "TypeScript +1" }]);
});
