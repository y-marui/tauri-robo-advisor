import assert from "node:assert/strict";
import { test } from "node:test";
import { bulkCategoryEdit, NEW_CATEGORY, productsOfCategory } from "./bulk-category.ts";
import type { Product } from "../types.ts";

const product = (id: number, category: number): Product => ({
  id,
  name: `Product ${id}`,
  category,
  currency: "JPY",
  aliases: [],
});

test("productsOfCategory picks the shown products of the category", () => {
  const all = [product(1, 1), product(2, 2), product(3, 1), product(4, 1)];
  assert.deepEqual(productsOfCategory(all, 1, new Set([1, 2, 3])), [1, 3]);
});

test("bulkCategoryEdit needs a selection", () => {
  assert.equal(bulkCategoryEdit([], "1", ""), null);
});

test("bulkCategoryEdit sets an existing category", () => {
  assert.deepEqual(bulkCategoryEdit([1, 2], "5", "ignored"), {
    type: "set_products_category",
    products: [1, 2],
    category: 5,
  });
});

test("bulkCategoryEdit makes a new category from a trimmed, non-empty name", () => {
  assert.equal(bulkCategoryEdit([1], NEW_CATEGORY, "   "), null);
  assert.deepEqual(bulkCategoryEdit([1], NEW_CATEGORY, " Gold "), {
    type: "set_products_category",
    products: [1],
    new_category: "Gold",
  });
});
