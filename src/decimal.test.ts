// Run with `npm test` (Node's built-in test runner strips the types).
import assert from "node:assert/strict";
import { test } from "node:test";

import {
  cleanDecimal,
  formatAmount,
  fractionToPercent,
  percentToFraction,
  shiftDecimal,
  sumDecimals,
} from "./decimal.ts";

test("percentToFraction moves the point two places left without rounding", () => {
  assert.equal(percentToFraction("30"), "0.3");
  assert.equal(percentToFraction("12.5"), "0.125");
  assert.equal(percentToFraction("0.5"), "0.005");
  assert.equal(percentToFraction("100"), "1");
  assert.equal(percentToFraction("0"), "0");
  assert.equal(percentToFraction("33.33"), "0.3333");
  assert.equal(percentToFraction(" 7 "), "0.07");
});

test("fractionToPercent is the exact inverse", () => {
  assert.equal(fractionToPercent("0.3"), "30");
  assert.equal(fractionToPercent("0.125"), "12.5");
  assert.equal(fractionToPercent("0.005"), "0.5");
  assert.equal(fractionToPercent("1"), "100");
  assert.equal(fractionToPercent("0"), "0");
  assert.equal(fractionToPercent("0.3333"), "33.33");
});

test("shifting is exact where floating point would not be", () => {
  // 0.1 + 0.2 style traps: the text is never converted to a number.
  assert.equal(shiftDecimal("0.29", 2), "29");
  assert.equal(shiftDecimal("1.005", 3), "1005");
  assert.equal(shiftDecimal("79228162514264337593543950335", -2), "792281625142643375935439503.35");
});

test("leading and trailing zeros are trimmed and a bare point is accepted", () => {
  assert.equal(shiftDecimal("007.500", 0), "7.5");
  assert.equal(shiftDecimal(".5", 0), "0.5");
  assert.equal(shiftDecimal("5.", 0), "5");
  assert.equal(shiftDecimal("-0", 0), "0");
  assert.equal(shiftDecimal("-2.50", 0), "-2.5");
});

test("text that is not a plain decimal is rejected", () => {
  for (const text of ["", " ", ".", "-", "abc", "1e3", "1,000", "1.2.3", "--1", "+5", "１２"]) {
    assert.equal(shiftDecimal(text, 0), null, JSON.stringify(text));
    assert.equal(cleanDecimal(text), null, JSON.stringify(text));
    assert.equal(percentToFraction(text), null, JSON.stringify(text));
  }
});

test("fractionToPercent leaves unparseable text unchanged", () => {
  assert.equal(fractionToPercent("n/a"), "n/a");
});

test("formatAmount groups digits by locale and keeps exact digits", () => {
  assert.equal(formatAmount("1234567", "en"), "1,234,567");
  assert.equal(formatAmount("1234567.89", "ja"), "1,234,567.89");
  assert.equal(formatAmount("12345678901234567890.123", "en"), "12,345,678,901,234,567,890.123");
  assert.equal(formatAmount("10.50", "en"), "10.50");
  assert.equal(formatAmount("-1000.00", "ja"), "-1,000.00");
  assert.doesNotThrow(() => formatAmount("1.1234567890123456789012345", "en"));
  assert.equal(formatAmount("not a number", "en"), "not a number");
});

test("sumDecimals adds exactly and refuses what is not a non-negative decimal", () => {
  assert.equal(sumDecimals(["1", "2"]), "3");
  assert.equal(sumDecimals(["0.1", "0.2"]), "0.3");
  assert.equal(sumDecimals(["1.5", "1.5"]), "3");
  assert.equal(sumDecimals(["0.0000009", "1"]), "1.0000009");
  assert.equal(sumDecimals(["0.4"]), "0.4");
  assert.equal(sumDecimals([]), null);
  assert.equal(sumDecimals(["1", "x"]), null);
  assert.equal(sumDecimals(["1", "-1"]), null);
});
