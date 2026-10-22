import assert from "node:assert/strict";
import { test } from "node:test";
import { weightOf } from "./product-shares.ts";
import type { PortfolioView } from "./types.ts";

const view = {
  portfolio: {
    product_shares: [
      { mode: "fixed_add", account: 7, product: 1, weight: "3" },
      { mode: "fixed_add", tab: 7, account: 7, product: 1, weight: "5" },
      { mode: "fixed_add", account: 7, product: 1, account_type: "nisa", weight: "2" },
      { mode: "fixed_sell", account: 7, product: 1, weight: "0.5" },
    ],
  },
} as unknown as PortfolioView;

test("each mode has its own weight and a missing one is 1", () => {
  assert.equal(weightOf(view, "fixed_add", null, 7, 1, null), "3");
  assert.equal(weightOf(view, "fixed_sell", null, 7, 1, null), "0.5");
  assert.equal(weightOf(view, "full_rebalance", null, 7, 1, null), "1");
  assert.equal(weightOf(view, "fixed_add", null, 7, 2, null), "1");
  assert.equal(weightOf({ portfolio: {} } as unknown as PortfolioView, "fixed_add", null, 7, 2, null), "1");
});

test("the All tab and each account tab have their own weight", () => {
  assert.equal(weightOf(view, "fixed_add", 7, 7, 1, null), "5");
  assert.equal(weightOf(view, "fixed_add", 8, 7, 1, null), "1");
  assert.equal(weightOf(view, "fixed_sell", 7, 7, 1, null), "1");
});

test("each record type of a product has its own weight", () => {
  assert.equal(weightOf(view, "fixed_add", null, 7, 1, "nisa"), "2");
  assert.equal(weightOf(view, "fixed_add", null, 7, 1, "general"), "1");
  assert.equal(weightOf(view, "fixed_add", null, 8, 1, null), "1");
});
