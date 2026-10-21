import assert from "node:assert/strict";
import { test } from "node:test";
import { clearAll, heldProducts, selectAll, toggle } from "./rebalance-selection.ts";
import type { PortfolioView } from "./types.ts";

const view = {
  portfolio: {
    products: [{ id: 1 }, { id: 2 }, { id: 3 }],
    holdings: [
      { account: 5, product: 3, value: "100" },
      { account: 5, product: 1, value: "0.50" },
      { account: 5, product: 2, value: "0" },
      { account: 6, product: 2, value: "10" },
    ],
  },
} as unknown as PortfolioView;

test("heldProducts lists what the account holds, in product order", () => {
  assert.deepEqual(heldProducts(view, 5), [1, 3]);
  assert.deepEqual(heldProducts(view, 6), [2]);
  assert.deepEqual(heldProducts(view, 9), []);
});

test("select all and clear only touch the shown products", () => {
  assert.deepEqual(selectAll([1, 2, 9], [1, 2, 3]), [9]);
  assert.deepEqual(clearAll([9], [1, 2]), [9, 1, 2]);
  assert.deepEqual(clearAll([1], [1, 2]), [1, 2]);
});

test("toggle switches one product", () => {
  assert.deepEqual(toggle([1, 2], 2, true), [1]);
  assert.deepEqual(toggle([1], 2, false), [1, 2]);
});
