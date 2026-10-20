import assert from "node:assert/strict";
import { test } from "node:test";
import { mergeClashes, mergeTargets, splitEdit } from "./merge-split.ts";
import type { PortfolioView, Product } from "../types.ts";

const product = (id: number, currency: string): Product => ({
  id,
  name: `Product ${id}`,
  category: 1,
  currency,
  aliases: [],
});

test("mergeTargets lists the other products of the same currency", () => {
  const all = [product(1, "JPY"), product(2, "JPY"), product(3, "USD")];
  assert.deepEqual(mergeTargets(all, all[0]!).map((p) => p.id), [2]);
});

test("mergeClashes finds the accounts holding both products", () => {
  const view = {
    portfolio: {
      holdings: [
        { account: 1, product: 1 },
        { account: 2, product: 1 },
        { account: 2, product: 2 },
        { account: 3, product: 2 },
      ],
    },
  } as unknown as PortfolioView;
  assert.deepEqual(mergeClashes(view, 1, 2), [2]);
});

test("splitEdit needs an alias and a name", () => {
  const alias = { source: "Source", name: "Name 1" };
  assert.equal(splitEdit(1, undefined, "New", []), null);
  assert.equal(splitEdit(1, alias, "  ", []), null);
  assert.deepEqual(splitEdit(1, alias, " New ", [2]), {
    type: "split_product",
    product: 1,
    source: "Source",
    alias: "Name 1",
    name: "New",
    accounts: [2],
  });
});
