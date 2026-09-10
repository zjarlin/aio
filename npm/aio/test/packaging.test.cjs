"use strict";

const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const path = require("node:path");
const test = require("node:test");

test("入口包包含选择器和完整许可证", () => {
  const packageRoot = path.resolve(__dirname, "..");
  const packed = JSON.parse(
    execFileSync("npm", ["pack", "--dry-run", "--json", packageRoot], {
      encoding: "utf8"
    })
  )[0];
  const packedFiles = packed.files.map((file) => file.path);

  assert.ok(packedFiles.includes("bin/aio.cjs"));
  assert.ok(packedFiles.includes("lib/platform.cjs"));
  assert.ok(packedFiles.includes("LICENSE-APACHE"));
  assert.ok(packedFiles.includes("LICENSE-MIT"));
});
