import assert from "node:assert/strict";
import { test } from "node:test";
import { isNameConflict, offerFirst, suggestedProduct, suggestionFor } from "./link-hints.ts";

const suggestions = [
  { name: "AAA", exact: 2, similar: null },
  { name: "AAB", exact: null, similar: 3 },
];

test("suggestionFor finds a name after trimming", () => {
  assert.equal(suggestionFor(suggestions, " AAB ")?.similar, 3);
  assert.equal(suggestionFor(suggestions, "ZZZ"), undefined);
  assert.equal(suggestionFor(undefined, "AAA"), undefined);
});

test("suggestedProduct prefers the same name to a similar one", () => {
  assert.equal(suggestedProduct(suggestions[0]), 2);
  assert.equal(suggestedProduct(suggestions[1]), 3);
  assert.equal(suggestedProduct(undefined), undefined);
});

test("offerFirst moves one item to the front and keeps the rest in order", () => {
  const items = [{ id: 1 }, { id: 2 }, { id: 3 }];
  assert.deepEqual(offerFirst(items, 3), [{ id: 3 }, { id: 1 }, { id: 2 }]);
  assert.deepEqual(offerFirst(items, 1), items);
  assert.deepEqual(offerFirst(items, 9), items);
  assert.deepEqual(offerFirst(items, undefined), items);
});

test("isNameConflict recognises only the name conflict", () => {
  assert.ok(isNameConflict("Error: conflict: a product with this name exists"));
  assert.ok(!isNameConflict("conflict: move or remove the category's products first"));
});
