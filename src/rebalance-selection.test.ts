import assert from "node:assert/strict";
import { test } from "node:test";
import { clearAll, selectAll, toggle } from "./rebalance-selection.ts";

test("select all and clear only touch the shown products", () => {
  assert.deepEqual(selectAll([1, 2, 9], [1, 2, 3]), [9]);
  assert.deepEqual(clearAll([9], [1, 2]), [9, 1, 2]);
  assert.deepEqual(clearAll([1], [1, 2]), [1, 2]);
});

test("toggle switches one product", () => {
  assert.deepEqual(toggle([1, 2], 2, true), [1]);
  assert.deepEqual(toggle([1], 2, false), [1, 2]);
});
