import assert from "node:assert/strict";
import { test } from "node:test";
import {
  loadExchangeAccount,
  pickAccount,
  saveExchangeAccount,
  type KeyValueStore,
} from "./exchange-account.ts";

function memoryStore(): KeyValueStore {
  const map = new Map<string, string>();
  return {
    getItem: (k) => map.get(k) ?? null,
    setItem: (k, v) => void map.set(k, v),
  };
}

test("the chosen account is remembered per exchange", () => {
  const store = memoryStore();
  saveExchangeAccount("bitflyer", "3", store);
  saveExchangeAccount("coincheck", "5", store);
  assert.equal(loadExchangeAccount("bitflyer", store), "3");
  assert.equal(loadExchangeAccount("coincheck", store), "5");
  assert.equal(loadExchangeAccount("gmocoin", store), null);
});

test("unavailable or failing storage is tolerated", () => {
  assert.equal(loadExchangeAccount("bitflyer", null), null);
  saveExchangeAccount("bitflyer", "3", null);
  const broken: KeyValueStore = {
    getItem: () => {
      throw new Error("blocked");
    },
    setItem: () => {
      throw new Error("blocked");
    },
  };
  assert.equal(loadExchangeAccount("bitflyer", broken), null);
  saveExchangeAccount("bitflyer", "3", broken);
});

test("pickAccount prefers the session choice, then the saved one", () => {
  assert.equal(pickAccount(["1", "2", "3"], "2", "3"), "2");
  assert.equal(pickAccount(["1", "2", "3"], null, "3"), "3");
  assert.equal(pickAccount(["1", "2", "3"], "", "3"), "3");
});

test("pickAccount ignores accounts that no longer exist", () => {
  assert.equal(pickAccount(["1", "2"], "9", "2"), "2");
  assert.equal(pickAccount(["1", "2"], "9", "8"), null);
  assert.equal(pickAccount([], null, "1"), null);
});
