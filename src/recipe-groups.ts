import { t, type MessageKey } from "./i18n.ts";
import type { PresetInfo, RecipeView } from "./types.ts";

// Pure helpers of the fetch recipes card: how a recipe is named and how the list is grouped.

/** One site's recipes under a heading; `site` is empty for recipes that read the page chosen on the screen. */
export interface SiteGroup {
  site: string;
  /** The company's name when a preset says it, else the site itself; empty for no site. */
  title: string;
  recipes: RecipeView[];
}

/**
 * The name shown for a recipe, the same in the list and in the picker: for a recipe of a preset
 * that still carries the preset's own name, the preset's (translated) label; else its own name.
 */
export function recipeDisplayName(recipe: { name: string; preset?: string | null }, presets: PresetInfo[]): string {
  const preset = presets.find((p) => p.id === recipe.preset);
  return preset !== undefined && preset.label === recipe.name ? t(preset.label as MessageKey) : recipe.name;
}

/** Groups recipes by the site they read, in the order the sites first appear; no-site recipes come last. */
export function groupBySite(recipes: RecipeView[], presets: PresetInfo[]): SiteGroup[] {
  const groups: SiteGroup[] = [];
  for (const recipe of recipes) {
    const site = recipe.start_site ?? "";
    let group = groups.find((g) => g.site === site);
    if (group === undefined) {
      const company = presets.find((p) => p.site === site && p.company !== "")?.company;
      group = { site, title: site === "" ? "" : (company ?? site), recipes: [] };
      groups.push(group);
    }
    group.recipes.push(recipe);
  }
  return groups.sort((a, b) => Number(a.site === "") - Number(b.site === ""));
}

/** Whether a tab of `site` (the site itself or a host below it) is among the open hosts. */
export function siteIsOpen(site: string, hosts: string[]): boolean {
  return hosts.some((host) => host === site || host.endsWith(`.${site}`));
}

/** The sites among the recipes' that have no open tab, each once, in order. */
export function sitesToLogIn(recipes: { start_site?: string }[], hosts: string[]): string[] {
  const sites: string[] = [];
  for (const recipe of recipes) {
    const site = recipe.start_site ?? "";
    if (site !== "" && !sites.includes(site) && !siteIsOpen(site, hosts)) {
      sites.push(site);
    }
  }
  return sites;
}
