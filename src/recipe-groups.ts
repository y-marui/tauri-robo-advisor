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

/**
 * The recipes to run after the login pages were handled: those of a site that had no open tab when
 * the fetch started are held back (the person has to log in first), except for a site with no
 * known login page (`unknown`), which is tried and reports its own problem.
 */
export function recipesAfterLogin<T extends { start_site?: string }>(
  recipes: T[],
  toLogIn: string[],
  unknown: string[],
): { run: T[] } {
  const held = toLogIn.filter((site) => !unknown.includes(site));
  return { run: recipes.filter((r) => !held.includes(r.start_site ?? "")) };
}

/** Adds a hint to a failure that says the first link was not found: the person may not be logged in. */
export function withLoginHint(reason: string): string {
  return reason.includes("no link with that text was found")
    ? `${reason} ${t("You may not be logged in: log in in the browser, then press Fetch again.")}`
    : reason;
}

/** The presets that no recipe refers to yet (for any account), in the order given. */
export function availablePresets(presets: PresetInfo[], recipes: { preset?: string | null }[]): PresetInfo[] {
  return presets.filter((p) => !recipes.some((r) => r.preset === p.id));
}

/** Whether "Add" may be pressed: a preset and an account are chosen, or a new account has a name. */
export function canAddPreset(preset: string, account: string, newAccountName: string): boolean {
  if (preset === "" || account === "") {
    return false;
  }
  return account !== "new" || newAccountName.trim() !== "";
}

/** Which earlier results a refresh drops when sites are held back for a login: all for a full run, else the held recipes'. */
export function resultsToDrop(fullRun: boolean, heldNames: string[]): { clearAll: boolean; discard: string[] } {
  return fullRun ? { clearAll: true, discard: [] } : { clearAll: false, discard: heldNames };
}
