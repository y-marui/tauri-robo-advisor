import assert from "node:assert/strict";
import { test } from "node:test";

import { setLanguage } from "./i18n.ts";
import { groupBySite, recipeDisplayName, siteIsOpen, sitesToLogIn } from "./recipe-groups.ts";
import type { PresetInfo, RecipeView } from "./types.ts";

const preset = (id: string, label: string, site: string, company: string): PresetInfo => ({
  id,
  label,
  description: "",
  table_starts_with: "",
  source: "",
  links: [],
  site,
  company,
});
const recipe = (name: string, start_site: string, account = 1): RecipeView =>
  ({ name, start_site, account, preset: null, preset_missing: false }) as unknown as RecipeView;

const presets = [preset("a", "Alpha Securities: stocks", "alpha.example", "Alpha Securities")];

test("a preset recipe shows the same name as the picker, an own recipe its own name", () => {
  setLanguage("en");
  assert.equal(recipeDisplayName({ name: "Alpha Securities: stocks", preset: "a" }, presets), "Alpha Securities: stocks");
  assert.equal(recipeDisplayName({ name: "Alpha Securities: stocks (Main)", preset: "a" }, presets), "Alpha Securities: stocks (Main)");
  assert.equal(recipeDisplayName({ name: "Mine", preset: null }, presets), "Mine");
});

test("recipes are grouped by site, the siteless ones last, with the company as the title", () => {
  const groups = groupBySite(
    [recipe("x", ""), recipe("one", "alpha.example"), recipe("two", "beta.example"), recipe("three", "alpha.example")],
    presets,
  );
  assert.deepEqual(
    groups.map((g) => [g.site, g.title, g.recipes.map((r) => r.name)]),
    [
      ["alpha.example", "Alpha Securities", ["one", "three"]],
      ["beta.example", "beta.example", ["two"]],
      ["", "", ["x"]],
    ],
  );
});

test("a site is open when a tab is on it or below it, and only closed sites need a login", () => {
  assert.ok(siteIsOpen("alpha.example", ["www.alpha.example"]));
  assert.ok(!siteIsOpen("alpha.example", ["badalpha.example"]));
  assert.deepEqual(
    sitesToLogIn([recipe("a", "alpha.example"), recipe("b", "alpha.example"), recipe("c", "beta.example"), recipe("d", "")], ["www.beta.example"]),
    ["alpha.example"],
  );
});
