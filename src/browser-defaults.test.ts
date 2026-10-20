import assert from "node:assert/strict";
import { test } from "node:test";
import { pickBrowser } from "./browser-defaults.ts";

test("pickBrowser uses the remembered browser while it is still found", () => {
  assert.equal(pickBrowser(["chrome", "brave"], "brave"), "brave");
});

test("pickBrowser falls back to the first one found", () => {
  assert.equal(pickBrowser(["edge", "brave"], "chrome"), "edge");
  assert.equal(pickBrowser(["edge", "brave"], null), "edge");
});

test("pickBrowser gives nothing when no browser is found", () => {
  assert.equal(pickBrowser([], "chrome"), null);
});
