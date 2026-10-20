import assert from "node:assert/strict";
import { test } from "node:test";

import { setLanguage } from "./i18n.ts";
import { BUILTIN_TYPES, matchesTypeFilter, NO_TYPE, typeFilterKey, typeLabel, typeOptions } from "./views/labels.ts";

test("the built-in types are offered in order, after “not set”, with Japanese names", () => {
  setLanguage("ja");
  assert.deepEqual(
    typeOptions(undefined).map((o) => o.label),
    ["未設定", "一般", "特定", "NISA（区分未定）", "NISA成長投資枠", "NISAつみたて投資枠", "旧NISA", "現金", "暗号資産"],
  );
  assert.equal(BUILTIN_TYPES.length, 8);
  setLanguage("en");
  assert.equal(typeLabel("deposit", undefined), "Cash");
  assert.equal(typeLabel("nisa_growth", undefined), "NISA growth");
  assert.equal(typeLabel("nisa_tsumitate", undefined), "NISA tsumitate");
  assert.equal(typeLabel("custom-7", [{ id: "custom-7", name: "iDeCo", sellable: true }]), "iDeCo");
});

test("the type filter shows every record, one type, or the records without a type", () => {
  assert.equal(matchesTypeFilter("nisa", ""), true);
  assert.equal(matchesTypeFilter(null, ""), true);
  assert.equal(matchesTypeFilter("nisa", "nisa"), true);
  assert.equal(matchesTypeFilter("specific", "nisa"), false);
  assert.equal(matchesTypeFilter(null, NO_TYPE), true);
  assert.equal(matchesTypeFilter(undefined, NO_TYPE), true);
  assert.equal(matchesTypeFilter("nisa", NO_TYPE), false);
  assert.equal(typeFilterKey(null), NO_TYPE);
});
