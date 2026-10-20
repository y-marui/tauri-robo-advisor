import assert from "node:assert/strict";
import { test } from "node:test";
import { differenceLines } from "./differences.ts";
import { setLanguage } from "../i18n.ts";

test("differenceLines lists the first rows and counts the rest", () => {
  setLanguage("en");
  const rows = Array.from({ length: 10 }, (_, i) => ({
    name: `Stock ${i}`,
    value: "1000",
    computed: "1200",
  }));
  const lines = differenceLines(rows);
  assert.equal(lines.length, 9);
  assert.equal(lines[0], "Stock 0: read 1,000, quantity x price 1,200");
  assert.equal(lines[8], "… (2)");
  assert.deepEqual(differenceLines([]), []);
});
