import { test } from 'node:test';
import assert from 'node:assert/strict';
import { increment } from '../dist/counter.mjs';
test('counter rejects invalid inputs and preserves integer semantics',()=>{
  assert.equal(increment(5),6);
  for(const value of [-1,0.5,NaN,Number.MAX_SAFE_INTEGER])assert.throws(()=>increment(value));
});
