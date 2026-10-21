import assert from "node:assert/strict";
import { test } from "node:test";
import { accountFor } from "./rebalance-account.ts";

test("an account tab works on its own account", () => {
  assert.equal(accountFor(3, 7), 3);
  assert.equal(accountFor(3, null), 3);
});

test("the All tab works on the rebalancing account, or none", () => {
  assert.equal(accountFor("all", 7), 7);
  assert.equal(accountFor("all", null), null);
});
