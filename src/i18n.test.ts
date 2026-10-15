import assert from "node:assert/strict";
import { test } from "node:test";

import { detectLanguage, MESSAGES, setLanguage, t, type MessageKey } from "./i18n.ts";

const placeholders = (text: string): string[] =>
  [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1] as string).sort();

test("every message has a non-empty Japanese translation with the same placeholders", () => {
  for (const [key, value] of Object.entries(MESSAGES)) {
    assert.ok(value.ja.trim().length > 0, key);
    assert.deepEqual(placeholders(value.ja), placeholders(key), key);
  }
});

test("English uses the key itself and Japanese uses the translation", () => {
  setLanguage("en");
  assert.equal(t("Categories"), "Categories");
  setLanguage("ja");
  assert.equal(t("Categories"), "カテゴリ");
  setLanguage("en");
});

test("placeholders are replaced and unknown ones are left visible", () => {
  setLanguage("en");
  assert.equal(
    t("Category targets add up to {sum}%, not 100%.", { sum: "90" }),
    "Category targets add up to 90%, not 100%.",
  );
  assert.equal(
    t("Could not save: {reason}", {}),
    "Could not save: {reason}",
  );
  const key: MessageKey = "Saved.";
  assert.equal(t(key), "Saved.");
});

test("the language is the saved choice, else Japanese for ja browsers, else English", () => {
  assert.equal(detectLanguage("ja", "en-US"), "ja");
  assert.equal(detectLanguage("en", "ja-JP"), "en");
  assert.equal(detectLanguage(null, "ja-JP"), "ja");
  assert.equal(detectLanguage(null, "fr-FR"), "en");
  assert.equal(detectLanguage("xx", "fr-FR"), "en");
});
