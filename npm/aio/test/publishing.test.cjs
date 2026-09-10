"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const { isMissingPackage } = require("../scripts/publish-packages.cjs");

function registryError(code) {
  return {
    stdout: JSON.stringify({ error: { code } })
  };
}

test("仅将 registry E404 识别为未发布", () => {
  assert.equal(isMissingPackage(registryError("E404")), true);
  assert.equal(isMissingPackage(registryError("E401")), false);
  assert.equal(isMissingPackage({ stdout: "invalid response" }), false);
});
