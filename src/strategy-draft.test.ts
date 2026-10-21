import assert from "node:assert/strict";
import { test } from "node:test";

import { strategyFromDraft } from "./strategy-draft.ts";

test("nothing ticked is not a strategy", () => {
  assert.deepEqual(strategyFromDraft([{ id: 1, checked: false, ratio: "3" }]), { ok: false, problem: "no_product" });
});

test("blank ratios on every ticked product mean equal shares; unticked rows are ignored", () => {
  assert.deepEqual(
    strategyFromDraft([
      { id: 1, checked: true, ratio: "" },
      { id: 2, checked: true, ratio: " " },
      { id: 3, checked: false, ratio: "5" },
    ]),
    { ok: true, items: [{ product: 1, ratio: null }, { product: 2, ratio: null }] },
  );
});

test("a mix of blank and typed ratios waits until it is consistent", () => {
  assert.deepEqual(
    strategyFromDraft([
      { id: 1, checked: true, ratio: "3" },
      { id: 2, checked: true, ratio: "" },
    ]),
    { ok: false, problem: "mixed_ratios" },
  );
  assert.equal(
    strategyFromDraft([
      { id: 1, checked: true, ratio: "3" },
      { id: 2, checked: true, ratio: "2" },
    ]).ok,
    true,
  );
});

test("ratios must be positive numbers", () => {
  assert.deepEqual(strategyFromDraft([{ id: 1, checked: true, ratio: "abc" }]), { ok: false, problem: "invalid_number" });
  assert.deepEqual(strategyFromDraft([{ id: 1, checked: true, ratio: "0" }]), { ok: false, problem: "not_positive" });
  assert.deepEqual(strategyFromDraft([{ id: 1, checked: true, ratio: "-2" }]), { ok: false, problem: "not_positive" });
});
