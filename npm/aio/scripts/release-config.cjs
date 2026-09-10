"use strict";

const PLATFORMS = Object.freeze([
  {
    id: "darwin-arm64",
    name: "@addzero/aio-darwin-arm64",
    os: "darwin",
    cpu: "arm64",
    executable: "aio"
  },
  {
    id: "darwin-x64",
    name: "@addzero/aio-darwin-x64",
    os: "darwin",
    cpu: "x64",
    executable: "aio"
  },
  {
    id: "linux-arm64",
    name: "@addzero/aio-linux-arm64",
    os: "linux",
    cpu: "arm64",
    executable: "aio"
  },
  {
    id: "linux-x64",
    name: "@addzero/aio-linux-x64",
    os: "linux",
    cpu: "x64",
    executable: "aio"
  },
  {
    id: "win32-x64",
    name: "@addzero/aio-win32-x64",
    os: "win32",
    cpu: "x64",
    executable: "aio.exe"
  }
]);

module.exports = { PLATFORMS };
