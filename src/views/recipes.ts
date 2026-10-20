import { applyFetchAll, browserPages, clearFetchRuns, discardFetchRun, fetchAllPlan, listPresets, openLoginPages, runRecipe, setLinkReference } from "../api.ts";
import { availablePresets, canAddPreset, groupBySite, recipeDisplayName, recipesAfterLogin, siteIsOpen, sitesToLogIn, withLoginHint } from "../recipe-groups.ts";
import { h } from "../dom.ts";
import { deleteButton } from "../fields.ts";
import { t, type MessageKey } from "../i18n.ts";
import type { Context, Panel } from "../panel.ts";
import type { BrowserPage, PortfolioView, PresetInfo, Recipe, RecipeRun, RecipeView } from "../types.ts";
import { importPlanView } from "./import-plan.ts";
import { browserControls } from "./browser-controls.ts";
import { skippedText } from "./skipped.ts";

// Saved fetch recipes: after the user has logged in to the browser opened by the
// app, one button follows each recipe's links (plain links that only show another
// page), reads its table, and shows the result of all of them together. The
// holdings change only when the user confirms, once, at the end.

// Only one run drives the browser tab at a time, even if the panel is rebuilt (for example
// after a language change) while a batch is still running; a later run waits for the one before.
let running: Promise<void> = Promise.resolve();

/** The value of the account choice that makes a new account. */
const NEW_ACCOUNT = "new";

export function recipesPanel(ctx: Context): Panel {
  const list = h("div", { class: "recipe-list" });
  const runButton = h("button", { type: "button" }, t("Fetch all"));
  const message = h("p", { class: "note", role: "status" });
  const attention = h("div", { class: "recipe-attention" });
  const results = h("div", { class: "recipe-results", "aria-live": "polite" });

  // Recipes that ship with the app: pick one and an account, and a recipe that refers to the preset is added
  // (its definition is the app's current one, so it is read-only). "New account..." makes the account first.
  const presetSelect = h("select", { "aria-label": t("Recipe to add") });
  const presetAccount = h("select", { "aria-label": t("Account to update") });
  const presetAdd = h("button", { type: "button", disabled: true }, t("Add"));
  const presetNote = h("p", { class: "note" });
  const newAccountName = h("input", { type: "text", "aria-label": t("Name of a new account"), placeholder: t("New account name") });
  newAccountName.hidden = true;
  presetAccount.addEventListener("change", () => {
    newAccountName.hidden = presetAccount.value !== NEW_ACCOUNT;
    if (!newAccountName.hidden) {
      newAccountName.focus();
    }
  });
  const addForm = h(
    "div",
    { class: "recipe-add" },
    h("h3", {}, t("Add from a preset")),
    presetSelect,
    presetAccount,
    newAccountName,
    presetAdd,
    presetNote,
  );
  let presets: PresetInfo[] = [];
  let presetAccountsSignature = "";
  /** What to do when a recipe could not read its table: the page to open, as its preset says. */
  const whatToOpen = (recipe: Recipe): string => {
    const preset = presets.find((p) => p.id === recipe.preset);
    return preset
      ? t(preset.description as MessageKey)
      : t("Open the page that shows the table starting with “{title}” in the browser, then fetch this recipe again.", {
          title: recipe.cards?.heading ?? recipe.table_starts_with,
        });
  };
  const accountNameOf = (recipe: Recipe): string =>
    current?.portfolio.accounts.find((a) => a.id === recipe.account)?.name ?? "?";
  /** A problem note with the page to open and a button to fetch just this recipe again. */
  const problem = (recipe: Recipe, text: string): HTMLElement => {
    const again = h("button", { type: "button" }, t("Fetch this recipe again"));
    again.addEventListener("click", () => void runAll([recipe.name]));
    return h(
      "div",
      { class: "recipe-problem" },
      h("p", { class: "note error" }, text),
      h("p", { class: "note" }, whatToOpen(recipe)),
      again,
    );
  };
  /** Add stays disabled until a preset and an account (or a new account's name) are chosen. */
  const updateAdd = (): void => {
    presetAdd.disabled = !canAddPreset(presetSelect.value, presetAccount.value, newAccountName.value);
  };
  const showPresetNote = (): void => {
    const description = presets.find((p) => p.id === presetSelect.value)?.description;
    presetNote.textContent = description === undefined ? "" : t(description as MessageKey);
    updateAdd();
  };
  /** The presets not yet added as a recipe (for any account); "---" first, and only that (disabled) when none is left. */
  const showPresetChoices = (): void => {
    const previous = presetSelect.value;
    const left = availablePresets(presets, current?.recipes ?? []);
    presetSelect.replaceChildren(
      h("option", { value: "" }, "---"),
      ...left.map((p) => h("option", { value: p.id }, t(p.label as MessageKey))),
    );
    presetSelect.disabled = left.length === 0;
    presetSelect.value = left.some((p) => p.id === previous) ? previous : "";
    showPresetNote();
  };
  newAccountName.addEventListener("input", updateAdd);
  presetAccount.addEventListener("change", updateAdd);
  presetSelect.addEventListener("change", showPresetNote);
  const addPreset = async (): Promise<void> => {
    let account = Number(presetAccount.value);
    if (presetSelect.value === "" || presetAccount.value === "") {
      fail(t("Choose a recipe and an account first."));
      return;
    }
    if (presetAccount.value === NEW_ACCOUNT) {
      const name = newAccountName.value.trim();
      if (name === "") {
        fail(t("Enter a name for the new account."));
        return;
      }
      if (!(await ctx.apply({ type: "add_account", name })) || !current) {
        return;
      }
      // The newest account is the one just made.
      account = Math.max(...current.portfolio.accounts.map((a) => a.id));
      newAccountName.value = "";
      presetAccount.value = String(account);
      newAccountName.hidden = true;
    }
    if (Number.isFinite(account)) {
      await ctx.apply({ type: "add_preset_recipe", preset: presetSelect.value, account });
    }
  };
  presetAdd.addEventListener("click", () => {
    presetAdd.disabled = true;
    void addPreset().finally(() => {
      updateAdd();
    });
  });
  void listPresets()
    .then((found) => {
      presets = found;
      showPresetChoices();
      if (current) {
        renderList(current);
      }
    })
    .catch(() => undefined);

  let current: PortfolioView | null = null;
  let hasRuns = false;
  let sequence = 0;
  let applyButton: HTMLButtonElement | null = null;
  let refresh = 0;

  const info = (text: string): void => {
    message.classList.remove("error");
    message.textContent = text;
  };
  const fail = (text: string): void => {
    message.classList.add("error");
    message.textContent = text;
  };

  // The dedicated browser, started here as in "Fetch in browser" (same part, same remembered choice).
  const browser = browserControls({
    info,
    fail,
    onOpened: () => {
      info(t("Log in to your brokerage in that browser, then fetch the recipes."));
    },
    compact: true,
    onClosed: () => undefined,
  });

  /** Runs one recipe; returns a note for the user when it needs attention. */
  async function runOne(recipe: Recipe, page: string): Promise<HTMLElement | null> {
    const outcome = await runRecipe(recipe.name, page);
    if (outcome.status === "ready") {
      return null;
    }
    const link = recipe.links[outcome.link];
    const saveReference = async (reference: string | null): Promise<void> => {
      // Rust changes only this link's reference, and only if the recipe is still the one that
      // produced this question: an edit made meanwhile must not be put back.
      try {
        ctx.setView(await setLinkReference(recipe, outcome.link, reference), t("Saved."));
      } catch (error) {
        fail(String(error));
        return;
      }
      await runAll([recipe.name]);
    };
    if (outcome.status === "link_changed") {
      const again = h("button", { type: "button" }, t("Choose the link again"));
      again.addEventListener("click", () => void saveReference(null));
      return h(
        "div",
        { class: "add-row" },
        h("span", { class: "note error" }, t("The link “{text}” of “{recipe}” has changed since it was saved.", { text: link?.text ?? "?", recipe: recipe.name })),
        again,
      );
    }
    const select = h(
      "select",
      { "aria-label": t("Where the link goes") },
      ...outcome.choices.map((c) => h("option", { value: c.reference }, c.label === "" ? t("(the page handles it)") : c.label)),
    );
    const use = h("button", { type: "button" }, t("Use this link"));
    use.addEventListener("click", () => void saveReference(select.value));
    return h(
      "div",
      { class: "add-row" },
      h("span", { class: "note error" }, t("Several different links say “{text}” (recipe “{recipe}”). Choose where it goes.", { text: link?.text ?? "?", recipe: recipe.name })),
      select,
      use,
    );
  }

  /** Runs the named recipes (all when none are named) and shows the combined result. */
  function runAll(only?: string[]): Promise<void> {
    const next = running.then(() => runBatch(only));
    running = next.catch(() => undefined);
    return next;
  }

  async function runBatch(only?: string[]): Promise<void> {
    const view = current;
    if (!view) {
      return;
    }
    let browserUp = true;
    let pendingNote = "";
    let open: BrowserPage[] = await browserPages().catch(() => {
      browserUp = false;
      return [];
    });
    /**
     * The tabs a recipe may read: the open pages of its own site. A site whose pages cannot be
     * reached by a link can have several pages open at once (a list of holdings and a page of
     * cash balances): the recipe reads the tab that has its table.
     */
    const pagesFor = (recipe: Recipe): string[] => {
      const site = recipe.start_site ?? "";
      return open.filter((p) => siteIsOpen(site, [p.host])).map((p) => p.id);
    };
    // A recipe with no site (stored by an older version) cannot be fetched: it has to be removed.
    let wanted = view.recipes.filter((r) => (!only || only.includes(r.name)) && (r.start_site ?? "") !== "");
    if (wanted.length === 0) {
      info("");
      return;
    }
    // A site with no open page is not fetched: its login page is opened (the browser too, if
    // needed) and the person logs in by hand and presses Fetch again. The other sites go on.
    const toLogIn = sitesToLogIn(
      wanted.filter((r) => !r.preset_missing),
      open.map((p) => p.host),
    );
    if (toLogIn.length > 0) {
      try {
        if (!browserUp && !(await browser.ensureOpen())) {
          fail(t("No Chrome, Edge, or Brave was found on this computer."));
          return;
        }
        const result = await openLoginPages(toLogIn);
        open = await browserPages().catch(() => open);
        const parts: string[] = [];
        const held = recipesAfterLogin(wanted, toLogIn, result.unknown);
        if (held.run.length < wanted.length) {
          // Whether the login page was just opened or came back with the browser's saved tabs, the
          // person has not logged in yet: nothing is fetched for these sites.
          parts.push(t("Log in in the opened browser, then press Fetch again."));
        }
        if (result.unknown.length > 0) {
          parts.push(t("No login page is known for: {sites}. Log in and open a page of the site by hand, then press Fetch again.", { sites: result.unknown.join(", ") }));
        }
        wanted = held.run;
        if (wanted.length === 0) {
          info(parts.join(" "));
          return;
        }
        pendingNote = parts.join(" ");
      } catch (error) {
        fail(String(error));
        return;
      }
    }
    const mine = ++sequence;
    // Whatever is on screen is out of date from now on, and must not be applied or redrawn.
    refresh++;
    if (applyButton) {
      applyButton.disabled = true;
    }
    runButton.disabled = true;
    attention.replaceChildren();
    const outcome = { worked: [] as string[], attention: [] as string[], notTried: [] as string[] };
    try {
      // A full run starts from nothing; only a retry of named recipes keeps the others.
      if (!only) {
        await clearFetchRuns();
        hasRuns = false;
        results.replaceChildren();
      }
      for (const recipe of wanted) {
        info(t("Fetching “{recipe}”…", { recipe: recipe.name }));
        if (recipe.preset_missing) {
          // An error of this recipe only: the others still run, and the saved recipe is kept.
          await discardFetchRun(recipe.name).catch(() => undefined);
          attention.append(h("p", { class: "note error" }, t("“{recipe}” refers to a preset that this version of the app does not have. Remove the recipe.", { recipe: recipe.name })));
          outcome.attention.push(recipe.name);
          continue;
        }
        const pages = pagesFor(recipe);
        if (pages.length === 0) {
          // Its earlier result must not survive a retry that could not start.
          await discardFetchRun(recipe.name).catch(() => undefined);
          attention.append(
            problem(
              recipe,
              t("“{recipe}” ({account}) needs an open page of {site}: log in and open it in the browser first.", {
                recipe: recipe.name,
                account: accountNameOf(recipe),
                site: recipe.start_site ?? "",
              }),
            ),
          );
          outcome.attention.push(recipe.name);
          continue;
        }
        try {
          let note: HTMLElement | null = null;
          let tried = 0;
          for (const page of pages) {
            tried += 1;
            try {
              note = await runOne(recipe, page);
              break;
            } catch (error) {
              // This tab has no table for the recipe: another tab of the site may have it.
              if (String(error).includes("No table on the page starts with") && tried < pages.length) {
                continue;
              }
              throw error;
            }
          }
          if (mine !== sequence) {
            return;
          }
          if (note) {
            attention.append(note);
            outcome.attention.push(recipe.name);
          } else {
            outcome.worked.push(recipe.name);
          }
        } catch (error) {
          const reason = String(error);
          attention.append(
            problem(
              recipe,
              t("“{recipe}” ({account}) failed: {reason}", {
                recipe: recipe.name,
                account: accountNameOf(recipe),
                reason: withLoginHint(reason.replace("[stop] ", "")),
              }),
            ),
          );
          outcome.attention.push(recipe.name);
          if (reason.includes("[stop] ")) {
            // The tab is not where the recipes expect it: the next one would read the wrong page.
            outcome.notTried = wanted.slice(wanted.indexOf(recipe) + 1).map((r) => r.name);
            break;
          }
        }
      }
      await showResults();
      info(pendingNote);
      // Each recipe has its own result; the ones that worked are kept.
      const lines: string[] = [];
      if (outcome.worked.length > 0) {
        lines.push(t("Worked: {names}.", { names: outcome.worked.join(", ") }));
      }
      if (outcome.attention.length > 0) {
        lines.push(t("Needs attention: {names}.", { names: outcome.attention.join(", ") }));
      }
      if (outcome.notTried.length > 0) {
        lines.push(t("Not tried: {names}.", { names: outcome.notTried.join(", ") }));
      }
      if (lines.length > 0 && wanted.length > 1) {
        attention.prepend(h("p", { class: "note" }, lines.join(" ")));
      }
    } finally {
      runButton.disabled = false;
    }
  }

  async function showResults(): Promise<void> {
    const view = current;
    if (!view) {
      return;
    }
    const mine = ++refresh;
    const runs: RecipeRun[] = await fetchAllPlan();
    if (mine !== refresh) {
      return; // a newer refresh has started; only the last one may show a result
    }
    hasRuns = runs.length > 0;
    if (runs.length === 0) {
      results.replaceChildren();
      return;
    }
    const accountName = (id: number): string =>
      view.portfolio.accounts.find((a) => a.id === id)?.name ?? "?";
    const sections = runs.map((run) => {
      const notes: HTMLElement[] = [];
      if (run.skipped.length > 0) {
        notes.push(
          h("p", { class: "note" }, skippedText(run.skipped)),
        );
      }
      return h(
        "div",
        { class: "recipe-run" },
        h("h3", {}, `${run.name} → ${accountName(run.account)}`),
        h("p", { class: "muted" }, t("{count} rows read", { count: String(run.rows_read) })),
        importPlanView({ ctx, view, plan: run.plan, source: run.source, currency: run.currency, notes }),
      );
    });
    const accounts = [...new Set(runs.map((r) => accountName(r.account)))].join(", ");
    const apply = h("button", { type: "button" }, t("Apply to the accounts"));
    applyButton = apply;
    let armed: number | undefined;
    apply.addEventListener("click", () => {
      if (armed === undefined) {
        apply.textContent = t("Confirm apply");
        apply.classList.add("danger", "armed");
        armed = window.setTimeout(() => {
          apply.textContent = t("Apply to the accounts");
          apply.classList.remove("danger", "armed");
          armed = undefined;
        }, 4000);
        return;
      }
      window.clearTimeout(armed);
      armed = undefined;
      apply.disabled = true;
      applyFetchAll()
        .then((next) => {
          hasRuns = false;
          results.replaceChildren();
          ctx.setView(next, t("The holdings were replaced."));
        })
        .catch((error) => ctx.fail(t("Could not apply: {reason}", { reason: String(error) })))
        .finally(() => {
          apply.disabled = false;
        });
    });
    results.replaceChildren(
      ...sections,
      h(
        "p",
        { class: "note" },
        t("This replaces every holding of these accounts with the linked rows above: {accounts}. An account without any linked row is left as it is.", { accounts }),
      ),
      apply,
    );
  }

  runButton.addEventListener("click", () => void runAll());

  /** A button that fetches just this recipe (the others keep what they already read). */
  function fetchOne(recipe: Recipe): HTMLElement {
    const button = h("button", { type: "button", title: t("Fetch this recipe") }, t("Fetch"));
    button.disabled = (recipe.start_site ?? "") === "";
    button.addEventListener("click", () => void runAll([recipe.name]));
    return button;
  }

  function renderList(view: PortfolioView): void {
    if (view.recipes.length === 0) {
      list.replaceChildren(
        h("p", { class: "note" }, t("No recipes yet. Add a built-in recipe below, or fetch a table once by hand in “Import a table” and use “Save as a recipe”.")),
      );
      runButton.disabled = true;
      list.append(addForm);
      return;
    }
    runButton.disabled = false;
    const groups = groupBySite(view.recipes, presets);
    const fetchAll = h("div", { class: "recipe-fetch-all" }, runButton);
    list.replaceChildren(
      fetchAll,
      ...groups.map((group) => {
        const siteButton = h("button", { type: "button" }, t("Fetch this site"));
        siteButton.addEventListener("click", () => void runAll(group.recipes.map((r) => r.name)));
        return h(
          "div",
          { class: "recipe-group" },
          group.site === ""
            ? h("h3", {}, t("No site"))
            : h("div", { class: "recipe-group-head" }, h("h3", {}, group.title), siteButton),
          group.site === ""
            ? h("p", { class: "note" }, t("These recipes name no site (an older version saved them), so they cannot be fetched. Delete them and save them again with a site."))
            : null,
          ...group.recipes.map((recipe) => recipeRow(view, recipe)),
        );
      }),
      addForm,
    );
  }

  /** A small "i" that shows `text` on hover and on keyboard focus (and to screen readers). */
  function infoIcon(text: string): HTMLElement {
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("viewBox", "0 0 24 24");
    svg.setAttribute("aria-hidden", "true");
    const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
    path.setAttribute("d", "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20zm1 15h-2v-6h2v6zm0-8h-2V7h2v2z");
    path.setAttribute("fill", "currentColor");
    svg.append(path);
    const icon = h("span", { class: "info-icon", tabindex: 0, role: "img", "aria-label": text, "data-tip": text });
    icon.append(svg);
    return icon;
  }

  /** The route of a recipe in a sentence: where it starts, the links it follows, and the table it reads. */
  function routeText(recipe: RecipeView): string {
    const preset = presets.find((p) => p.id === recipe.preset);
    if (preset) {
      return t(preset.description as MessageKey);
    }
    const route = `${recipe.links.map((l) => l.text).join(" › ") || t("(this page)")} › ${recipe.cards?.heading ?? recipe.table_starts_with}`;
    return `${route} (${recipe.currency === "" ? t("(the product’s currency)") : recipe.currency})`;
  }

  /** One compact row: name, account, and actions. A preset recipe is read-only (it can be fetched or removed). */
  function recipeRow(view: PortfolioView, recipe: RecipeView): HTMLElement {
    const remove = deleteButton(() => void ctx.apply({ type: "remove_recipe", name: recipe.name }));
    if (recipe.preset_missing) {
      return h(
        "div",
        { class: "add-row recipe-row" },
        h("span", { class: "note error" }, t("“{recipe}” refers to a preset that this version of the app does not have. Remove the recipe.", { recipe: recipe.name })),
        remove,
      );
    }
    const account = h(
      "select",
      { "aria-label": t("Account of {recipe}", { recipe: recipeDisplayName(recipe, presets) }) },
      ...view.portfolio.accounts.map((a) => h("option", { value: String(a.id) }, a.name)),
    );
    account.value = String(recipe.account);
    account.addEventListener("change", () => {
      void ctx
        .apply({ type: "set_recipe_account", name: recipe.name, account: Number(account.value) })
        .then((ok) => {
          if (!ok) {
            account.value = String(recipe.account);
          }
        });
    });
    return h(
      "div",
      { class: "add-row recipe-row" },
      h(
        "span",
        { class: "recipe-name" },
        h("span", {}, recipeDisplayName(recipe, presets)),
        h("span", { class: "recipe-tags" }, recipe.preset ? h("span", { class: "badge" }, t("built-in")) : null, infoIcon(routeText(recipe))),
      ),
      account,
      fetchOne(recipe),
      remove,
    );
  }

  const el = h(
    "section",
    { class: "card recipes" },
    h("h2", {}, t("Fetch recipes")),
    h(
      "p",
      { class: "note" },
      t("“Fetch” follows each recipe’s links (they only show other pages), reads its table, and shows the result. If a site has no open page, its login page is opened: log in by hand, then press Fetch again. Nothing changes until you confirm."),
    ),
    list,
    h("h3", { class: "browser-heading" }, t("Browser settings")),
    browser.el,
    message,
    attention,
    results,
  );

  void clearFetchRuns();

  return {
    el,
    render(view) {
      current = view;
      renderList(view);
      showPresetChoices();
      const accountsSignature = view.portfolio.accounts.map((a) => `${a.id}:${a.name}`).join("|");
      if (accountsSignature !== presetAccountsSignature) {
        presetAccountsSignature = accountsSignature;
        const previous = presetAccount.value;
        presetAccount.replaceChildren(
          h("option", { value: "" }, "---"),
          ...view.portfolio.accounts.map((a) => h("option", { value: String(a.id) }, a.name)),
          h("option", { value: NEW_ACCOUNT }, t("New account…")),
        );
        presetAccount.value =
          previous === NEW_ACCOUNT || view.portfolio.accounts.some((a) => String(a.id) === previous) ? previous : "";
        updateAdd();
      }
      // An alias was linked or removed, or a recipe changed: show the plans again.
      if (hasRuns) {
        // The result on screen is out of date until the plans are made again: it cannot be applied.
        if (applyButton) {
          applyButton.disabled = true;
        }
        void showResults().catch((error) => fail(String(error)));
      }
    },
  };
}
