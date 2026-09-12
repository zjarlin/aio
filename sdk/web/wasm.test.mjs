import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const source = readFileSync(new URL('./wasm.js', import.meta.url), 'utf8');
const bytes = new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]);
function runtime(methods = WebAssembly) {
  const api = Object.fromEntries(['compileStreaming', 'instantiateStreaming'].map(name => [name, methods[name].bind(methods)]));
  vm.runInNewContext(source, {WebAssembly: api, Response});
  return api;
}

test('downloads finish before compilation and preserve the response and options', async () => {
  let finish, compiled = false;
  const response = new Response(new ReadableStream({start(controller) {
    controller.enqueue(bytes.subarray(0, 4));
    finish = () => {controller.enqueue(bytes.subarray(4));controller.close();};
  }}), {headers: {'content-type': 'application/wasm'}});
  const imports = {}, options = {builtins: ['js-string']};
  const methods = {
    compileStreaming: WebAssembly.compileStreaming,
    async instantiateStreaming(value, actualImports, actualOptions) {
      compiled = true;
      assert.equal(value, response);
      assert.equal(actualImports, imports);
      assert.equal(actualOptions, options);
      assert.deepEqual(new Uint8Array(await value.arrayBuffer()), bytes);
      return 'compiled';
    },
  };
  const result = runtime(methods).instantiateStreaming(Promise.resolve(response), imports, options);
  await new Promise(resolve => setTimeout(resolve, 10));
  assert.equal(compiled, false);
  finish();
  assert.equal(await result, 'compiled');
});

test('native Wasm validation, HTTP errors and stream failures are retained', async () => {
  const api = runtime();
  const response = () => new Response(bytes, {headers: {'content-type': 'application/wasm'}});
  assert(await api.compileStreaming(response()) instanceof WebAssembly.Module);
  assert((await api.instantiateStreaming(response())).instance instanceof WebAssembly.Instance);
  await assert.rejects(api.compileStreaming(new Response(bytes)), TypeError);
  await assert.rejects(api.compileStreaming(new Response(bytes, {status: 403})), TypeError);
  const consumed = response();
  await consumed.arrayBuffer();
  await assert.rejects(api.compileStreaming(consumed), TypeError);
  await assert.rejects(api.compileStreaming(new Response(new ReadableStream({start(controller) {
    controller.error(new Error('download aborted'));
  }}), {headers: {'content-type': 'application/wasm'}})), /download aborted/);
});
