// Tests of the card reader script against a tiny fake DOM with made-up pages.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const source = readFileSync(new URL("./read_cards.js", import.meta.url), "utf8");

class El {
  constructor(tag, children = [], { text, hidden = false, empty = false } = {}) {
    this.tagName = tag.toUpperCase();
    this.children = children.filter((c) => c instanceof El);
    this.ownText = text ?? (typeof children[0] === "string" ? children.filter((c) => typeof c === "string").join("\n") : "");
    this.hidden = hidden;
    this.empty = empty; // zero-height: no client rects
    this.parentElement = null;
    for (const c of this.children) c.parentElement = this;
  }
  get innerText() {
    const own = this.ownText ? [this.ownText] : [];
    return [...own, ...this.children.filter((c) => !c.hidden).map((c) => c.innerText)].join("\n");
  }
  get nextElementSibling() {
    const siblings = this.parentElement?.children ?? [];
    return siblings[siblings.indexOf(this) + 1] ?? null;
  }
  getClientRects() {
    return this.empty ? [] : [{}];
  }
  all() {
    return [this, ...this.children.flatMap((c) => c.all())];
  }
}

const run = (root, mode, heading = "", depth = 1) => {
  const document = { querySelectorAll: () => root.all().filter((e) => /^H[1-6]$/.test(e.tagName)) };
  const style = (e) => ({ visibility: "visible", display: e.hidden ? "none" : "block", opacity: "1" });
  return new Function("document", "getComputedStyle", `return ${source}`)(document, style)(mode, heading, depth);
};
const card = (...lines) => new El("div", [lines.join("\n")]);
const page = (...parts) => new El("body", parts);

test("one matching heading: each block under the region is a row, each line a cell", () => {
  const root = page(new El("h2", ["Holdings (2)"]), new El("div", [card("Fund A", "1,000 yen"), card("Fund B", "2,000 yen", "+5")]));
  assert.deepEqual(run(root, "read", "Holdings"), { items: [["Fund A", "1,000 yen"], ["Fund B", "2,000 yen", "+5"]], cut: false });
});

test("no heading and several headings are errors", () => {
  const root = page(new El("h2", ["Holdings"]), new El("h3", ["Holdings again"]), new El("div", [card("a")]));
  assert.deepEqual(run(root, "read", "Nothing"), { error: "none" });
  assert.deepEqual(run(root, "read", "Holdings"), { error: "many" });
  assert.deepEqual(run(root, "read", ""), { error: "none" });
});

test("a hidden heading does not count", () => {
  const root = page(new El("h2", ["Holdings"], { hidden: true }), new El("h2", ["Holdings"]), new El("div", [card("a")]));
  assert.equal(run(root, "read", "Holdings").items.length, 1);
});

test("a heading alone in a wrapper stands for the wrapper", () => {
  const root = page(new El("div", [new El("div", [new El("h2", ["Holdings"])])]), new El("section", [card("a", "1"), card("b", "2")]));
  assert.equal(run(root, "read", "Holdings").items.length, 2);
});

test("the region stops at the next heading", () => {
  const root = page(new El("h2", ["Holdings"]), new El("h2", ["Other"]), new El("div", [card("x")]));
  assert.deepEqual(run(root, "read", "Holdings"), { items: [], cut: false });
});

test("a next section wrapped around its heading is not the region", () => {
  const root = page(new El("h2", ["Holdings"]), new El("section", [new El("h2", ["Other"]), new El("div", [card("x")])]));
  assert.deepEqual(run(root, "read", "Holdings"), { items: [], cut: false });
  // Cards that themselves begin with a heading cannot be told from a next section: refused (no items).
  const titled = new El("div", [new El("div", [new El("h3", ["t"]), card("a", "1")]), card("b", "2")]);
  assert.deepEqual(run(page(new El("h2", ["Holdings"]), titled), "read", "Holdings"), { items: [], cut: false });
  // A heading inside a card, after its first block, is fine.
  const later = new El("div", [new El("div", [card("a", "1"), new El("h3", ["t"])]), card("b", "2")]);
  assert.equal(run(page(new El("h2", ["Holdings"]), later), "read", "Holdings").items.length, 2);
});

test("a single child that holds several blocks asks for a deeper item depth", () => {
  const inner = new El("div", [card("a", "1"), card("b", "2")]);
  const root = page(new El("h2", ["Holdings"]), new El("div", [inner]));
  assert.deepEqual(run(root, "read", "Holdings", 1), { error: "shallow" });
  assert.equal(run(root, "read", "Holdings", 2).items.length, 2);
});

test("hidden, empty, and zero-height blocks are not items", () => {
  const list = new El("div", [card("a", "1"), new El("div", [card("h")], { hidden: true }), new El("div", [""]), new El("div", [card("z")], { empty: true }), card("b", "2")]);
  const root = page(new El("h2", ["Holdings"]), list);
  assert.deepEqual(run(root, "read", "Holdings").items, [["a", "1"], ["b", "2"]]);
});

test("limits cut the result and say so", () => {
  const many = new El("div", Array.from({ length: 501 }, (_, i) => card(`n${i}`)));
  assert.equal(run(page(new El("h2", ["H"]), many), "read", "H").cut, true);
  const tall = new El("div", [card(...Array.from({ length: 21 }, (_, i) => `l${i}`)), card("b")]);
  const out = run(page(new El("h2", ["H"]), tall), "read", "H");
  assert.equal(out.cut, true);
  assert.equal(out.items[0].length, 20);
  const wide = new El("div", [card("x".repeat(1001)), card("b")]);
  assert.equal(run(page(new El("h2", ["H"]), wide), "read", "H").cut, true);
});

test("the heading list is limited to 50 headings of 60 characters", () => {
  const headings = Array.from({ length: 51 }, (_, i) => new El("h3", [`${i} ${"x".repeat(70)}`]));
  const out = run(page(...headings), "list");
  assert.equal(out.headings.length, 50);
  assert.equal(out.cut, true);
  assert.equal(out.headings[0].length, 60);
});

test("the script uses no API that acts on the page", () => {
  for (const forbidden of ["click(", "submit(", "dispatchEvent", ".value =", ".href", "getAttribute", "fetch(", "XMLHttpRequest", "eval(", "Function(", "location", "innerHTML", "classList", "className", "focus("]) {
    assert.ok(!source.includes(forbidden), forbidden);
  }
  const used = new Set(source.match(/\b(querySelectorAll|querySelector|getComputedStyle|getClientRects|innerText|textContent)\b/g));
  for (const name of used) assert.ok(["querySelectorAll", "getComputedStyle", "getClientRects", "innerText", "textContent"].includes(name), name);
});
