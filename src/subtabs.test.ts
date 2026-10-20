import assert from "node:assert/strict";
import { test } from "node:test";
import { pickSubTab, stepSubTab } from "./subtabs.ts";

test("pickSubTab keeps a known choice and falls back to the first", () => {
  assert.equal(pickSubTab("rebalance"), "rebalance");
  assert.equal(pickSubTab("holdings"), "holdings");
  assert.equal(pickSubTab("gone"), "holdings");
  assert.equal(pickSubTab(null), "holdings");
});

test("pickSubTab maps the retired sub-tabs to Rebalance", () => {
  assert.equal(pickSubTab("suggestions"), "rebalance");
  assert.equal(pickSubTab("accounts"), "rebalance");
  assert.equal(pickSubTab("toString"), "holdings");
});

test("stepSubTab wraps around and ignores other keys", () => {
  assert.equal(stepSubTab("holdings", "ArrowRight"), "rebalance");
  assert.equal(stepSubTab("rebalance", "ArrowRight"), "holdings");
  assert.equal(stepSubTab("holdings", "ArrowLeft"), "rebalance");
  assert.equal(stepSubTab("holdings", "Enter"), null);
});
