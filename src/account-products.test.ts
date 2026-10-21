import assert from "node:assert/strict";
import { test } from "node:test";
import { productGroups } from "./account-products.ts";
import type { Account, AccountBalance, PortfolioView } from "./types.ts";

const view = {
  portfolio: {
    categories: [{ id: 1 }, { id: 2 }, { id: 3 }],
    products: [
      { id: 10, category: 1 },
      { id: 11, category: 1 },
      { id: 12, category: 2 },
    ],
  },
} as unknown as PortfolioView;

const balance = {
  account: 5,
  categories: [
    {
      category: 1,
      total: "300",
      products: [
        { product: 10, current: "300", target: "200", shortfall: "-100" },
        { product: 11, current: "0", target: "100", shortfall: "100" },
      ],
    },
    {
      category: 2,
      total: "80",
      products: [{ product: 12, current: "80", target: null, shortfall: null }],
    },
  ],
} as unknown as AccountBalance;

const account = { id: 5, excluded: [11] } as unknown as Account;

test("show all lists the products the account holds, with the figures Rust gave", () => {
  const groups = productGroups(view, account, balance, true);
  assert.deepEqual(groups.map((g) => g.category), [1, 2]);
  assert.deepEqual(groups[0]?.rows.map((r) => [r.product, r.current, r.takesPart]), [[10, "300", true]]);
  assert.equal(groups[0]?.rows[0]?.target, "200");
  assert.equal(groups[0]?.total, "300");
  assert.equal(groups[1]?.total, "80");
});

test("a held product left out is still listed, a product not held never is", () => {
  const groups = productGroups(view, { ...account, excluded: [10, 11] } as Account, balance, true);
  assert.deepEqual(groups[0]?.rows.map((r) => [r.product, r.takesPart]), [[10, false]]);
});

test("tradable only drops the products left out, and the categories that end up empty", () => {
  const groups = productGroups(view, { ...account, excluded: [12] } as Account, balance, false);
  assert.deepEqual(groups.map((g) => g.rows.map((r) => r.product)), [[10]]);
});

test("a product strategy lists the products it names in one group", () => {
  const strategy = {
    id: 6,
    excluded: [],
    strategy: { type: "products", items: [{ product: 12, ratio: null }, { product: 10, ratio: "2" }] },
  } as unknown as Account;
  const figures = {
    account: 6,
    categories: [],
    strategy: { total: "50", products: [{ product: 10, current: "50", target: "40", shortfall: "-10" }] },
  } as unknown as AccountBalance;

  const groups = productGroups(view, strategy, figures, true);

  assert.equal(groups.length, 1);
  assert.equal(groups[0]?.category, null);
  assert.equal(groups[0]?.total, "50");
  assert.deepEqual(groups[0]?.rows.map((r) => [r.product, r.current]), [[12, "0"], [10, "50"]]);
});

test("without a balance nothing is known to be held, so nothing is listed", () => {
  assert.deepEqual(productGroups(view, account, null, true), []);
});
