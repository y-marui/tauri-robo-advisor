import assert from "node:assert/strict";
import { test } from "node:test";
import { dropIndex, slotAt } from "./reorder.ts";

test("dropIndex gives the final place, and nothing for the same place", () => {
  // Rows A B C D: dragging B (1).
  assert.equal(dropIndex(1, 0, 4), 0);
  assert.equal(dropIndex(1, 1, 4), null);
  assert.equal(dropIndex(1, 2, 4), null);
  assert.equal(dropIndex(1, 3, 4), 2);
  assert.equal(dropIndex(1, 4, 4), 3);
  assert.equal(dropIndex(3, 0, 4), 0);
  assert.equal(dropIndex(0, 4, 4), 3);
});

test("dropIndex refuses places that do not exist", () => {
  assert.equal(dropIndex(-1, 0, 3), null);
  assert.equal(dropIndex(3, 0, 3), null);
  assert.equal(dropIndex(0, 4, 3), null);
  assert.equal(dropIndex(0, -1, 3), null);
});

test("slotAt picks the gap by the middles of the rows", () => {
  const middles = [10, 30, 50];
  assert.equal(slotAt(middles, 0), 0);
  assert.equal(slotAt(middles, 20), 1);
  assert.equal(slotAt(middles, 40), 2);
  assert.equal(slotAt(middles, 99), 3);
  assert.equal(slotAt([], 5), 0);
});
