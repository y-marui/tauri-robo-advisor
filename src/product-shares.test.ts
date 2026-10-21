import assert from "node:assert/strict";
import { test } from "node:test";
import { shareOf, switchKind } from "./product-shares.ts";
import type { PortfolioView } from "./types.ts";

const view = {
  portfolio: {
    product_shares: [
      { mode: "fixed_add", product: 1, share: { kind: "weighted", weight: "3" } },
      { mode: "fixed_sell", product: 1, share: { kind: "keep_current" } },
    ],
  },
} as unknown as PortfolioView;

test("each mode has its own setting and a missing one is an equal share", () => {
  assert.deepEqual(shareOf(view, "fixed_add", 1), { kind: "weighted", weight: "3" });
  assert.deepEqual(shareOf(view, "fixed_sell", 1), { kind: "keep_current" });
  assert.deepEqual(shareOf(view, "full_rebalance", 1), { kind: "equal" });
  assert.deepEqual(shareOf({ portfolio: {} } as unknown as PortfolioView, "fixed_add", 2), { kind: "equal" });
});

test("switching to weighted keeps the old weight or starts at 1", () => {
  assert.deepEqual(switchKind(view, "fixed_add", 1, "weighted"), {
    type: "set_product_share",
    mode: "fixed_add",
    product: 1,
    share: { kind: "weighted", weight: "3" },
  });
  assert.deepEqual(switchKind(view, "fixed_sell", 1, "weighted").share, { kind: "weighted", weight: "1" });
  assert.deepEqual(switchKind(view, "fixed_add", 1, "keep_current").share, { kind: "keep_current" });
});
