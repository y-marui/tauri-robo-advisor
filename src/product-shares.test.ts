import assert from "node:assert/strict";
import { test } from "node:test";
import { weightOf } from "./product-shares.ts";
import type { PortfolioView } from "./types.ts";

const view = {
  portfolio: {
    product_shares: [
      { mode: "fixed_add", product: 1, weight: "3" },
      { mode: "fixed_sell", product: 1, weight: "0.5" },
    ],
  },
} as unknown as PortfolioView;

test("each mode has its own weight and a missing one is 1", () => {
  assert.equal(weightOf(view, "fixed_add", 1), "3");
  assert.equal(weightOf(view, "fixed_sell", 1), "0.5");
  assert.equal(weightOf(view, "full_rebalance", 1), "1");
  assert.equal(weightOf(view, "fixed_add", 2), "1");
  assert.equal(weightOf({ portfolio: {} } as unknown as PortfolioView, "fixed_add", 2), "1");
});
