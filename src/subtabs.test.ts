import assert from "node:assert/strict";
import { test } from "node:test";
import { pickSubTab, stepSubTab } from "./subtabs.ts";

test("pickSubTab keeps a known choice and falls back to the first", () => {
  assert.equal(pickSubTab("suggestions"), "suggestions");
  assert.equal(pickSubTab("accounts"), "accounts");
  assert.equal(pickSubTab("gone"), "holdings");
  assert.equal(pickSubTab(null), "holdings");
});

test("stepSubTab wraps around and ignores other keys", () => {
  assert.equal(stepSubTab("holdings", "ArrowRight"), "suggestions");
  assert.equal(stepSubTab("accounts", "ArrowRight"), "holdings");
  assert.equal(stepSubTab("holdings", "ArrowLeft"), "accounts");
  assert.equal(stepSubTab("holdings", "Enter"), null);
});
