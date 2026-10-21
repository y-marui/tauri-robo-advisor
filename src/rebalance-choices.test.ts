import assert from "node:assert/strict";
import { test } from "node:test";
import type { KeyValueStore } from "./exchange-account.ts";
import { loadListMode, loadMethod, saveListMode, saveMethod } from "./rebalance-choices.ts";

function memory(): KeyValueStore {
  const data = new Map<string, string>();
  return { getItem: (k) => data.get(k) ?? null, setItem: (k, v) => void data.set(k, v) };
}

test("each tab keeps its own method and list mode", () => {
  const store = memory();
  saveMethod("all", "by_product_weights", store);
  saveMethod(3, "follow_portfolio", store);
  saveListMode(3, "tradable", store);
  assert.equal(loadMethod("all", store), "by_product_weights");
  assert.equal(loadMethod(3, store), "follow_portfolio");
  assert.equal(loadMethod(4, store), null);
  assert.equal(loadListMode(3, store), "tradable");
  assert.equal(loadListMode("all", store), null);
});

test("an unknown saved value and failing storage give nothing", () => {
  const store = memory();
  store.setItem("rebalance-method:all", "nonsense");
  assert.equal(loadMethod("all", store), null);
  const broken: KeyValueStore = {
    getItem: () => {
      throw new Error("blocked");
    },
    setItem: () => {
      throw new Error("blocked");
    },
  };
  assert.equal(loadMethod("all", broken), null);
  saveMethod("all", "follow_portfolio", broken);
  assert.equal(loadMethod("all", null), null);
});
