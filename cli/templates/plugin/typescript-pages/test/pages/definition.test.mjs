import assert from "node:assert/strict";
import test from "node:test";

import { definitionJson, pages } from "../../build/pages/definition.js";

test("exposes unique counter and text pages", () => {
  assert.equal(pages.length, 2);
  assert.equal(new Set(pages.map(({ id }) => id)).size, pages.length);
  assert.equal(pages[0].body.kind, "counter");
  assert.equal(pages[1].body.kind, "text");
});

test("encodes the host page definition contract", () => {
  const definitions = JSON.parse(definitionJson());

  assert.equal(definitions[0].id, __PRIMARY_ID__);
  assert.equal(definitions[0].required_permission, null);
  assert.equal(definitions[0].body.kind, "counter");
  assert.equal(definitions[1].body.kind, "text");
});
