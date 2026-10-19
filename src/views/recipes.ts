import { applyFetchAll, browserPages, clearFetchRuns, fetchAllPlan, listPresets, openLoginPages, runRecipe, setLinkReference } from "../api.ts";
import { h } from "../dom.ts";
import { deleteButton } from "../fields.ts";
import { t, type MessageKey } from "../i18n.ts";
import type { Context, Panel } from "../panel.ts";
import type { BrowserPage, PortfolioView, PresetInfo, Recipe, RecipeRun } from "../types.ts";
import { importPlanView } from "./import-plan.ts";

// Saved fetch recipes: after the user has logged in to the browser opened by the
// app, one button follows each recipe's links (plain links that only show another
// page), reads its table, and shows the result of all of them together. The
// holdings change only when the user confirms, once, at the end.

// Only one run drives the browser tab at a time, even if the panel is rebuilt (for example
// after a language change) while a batch is still running; a later run waits for the one before.
let running: Promise<void> = Promise.resolve();

export function recipesPanel(ctx: Context): Panel {
  const list = h("div", { class: "recipe-list" });
  const pageSelect = h("select", { "aria-label": t("Page") });
  const refreshButton = h("button", { type: "button" }, t("Refresh pages"));
  const runButton = h("button", { type: "button" }, t("Fetch everything"));
  const loginButton = h("button", { type: "button" }, t("Open the login pages"));
  const message = h("p", { class: "note", role: "status" });
  const attention = h("div", { class: "recipe-attention" });
  const results = h("div", { class: "recipe-results", "aria-live": "polite" });

  // Recipes that ship with the app: pick one and an account, and it is added as an ordinary recipe.
  const presetSelect = h("select", { "aria-label": t("Preset") });
  const presetAccount = h("select", { "aria-label": t("Account to update") });
  const presetAdd = h("button", { type: "button" }, t("Add the preset"));
  const presetNote = h("p", { class: "note" });
  const noAccountNote = h("p", { class: "note error" }, t("Add an account first (in the Accounts card), then add the preset."));
  noAccountNote.hidden = true;
  let presets: PresetInfo[] = [];
  let presetAccountsSignature = "";
  const showPresetNote = (): void => {
    const description = presets.find((p) => p.id === presetSelect.value)?.description;
    presetNote.textContent = description === undefined ? "" : t(description as MessageKey);
  };
  presetSelect.addEventListener("change", showPresetNote);
  presetAdd.addEventListener("click", () => {
    const account = Number(presetAccount.value);
    if (presetSelect.value === "" || presetAccount.value === "" || !Number.isFinite(account)) {
      fail(t("Choose a preset and an account first."));
      return;
    }
    void ctx.apply({ type: "add_preset_recipe", preset: presetSelect.value, account });
  });
  void listPresets()
    .then((found) => {
      presets = found;
      presetSelect.replaceChildren(...found.map((p) => h("option", { value: p.id }, t(p.label as MessageKey))));
      showPresetNote();
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

  async function refreshPages(): Promise<void> {
    let pages: BrowserPage[] = [];
    try {
      pages = await browserPages();
    } catch {
      // The browser is not open yet.
    }
    pageSelect.replaceChildren(
      ...pages.map((p) => h("option", { value: p.id }, `${p.host} — ${p.title || t("(no title)")}`)),
    );
  }

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
    const open: BrowserPage[] = await browserPages().catch(() => []);
    /** The tab a recipe reads: the open page of its own site, or the one chosen on the screen. */
    const pageFor = (recipe: Recipe): string | null => {
      if (recipe.start_site) {
        const site = recipe.start_site;
        const within = (p: BrowserPage): boolean => p.host === site || p.host.endsWith(`.${site}`);
        // With several tabs of the same site, the one chosen on the screen wins (the list of
        // holdings of a site that cannot be navigated is the page the user opened).
        const chosen = open.find((p) => p.id === pageSelect.value && within(p));
        return (chosen ?? open.find(within))?.id ?? null;
      }
      return pageSelect.value === "" ? null : pageSelect.value;
    };
    const wanted = view.portfolio.recipes.filter((r) => !only || only.includes(r.name));
    if (wanted.every((r) => !r.start_site) && pageSelect.value === "") {
      fail(t("Open the browser, log in, and open a page of the site first."));
      return;
    }
    const mine = ++sequence;
    // The tabs are fixed for the whole batch: the selection cannot change under it.
    pageSelect.disabled = true;
    refreshButton.disabled = true;
    // Whatever is on screen is out of date from now on, and must not be applied or redrawn.
    refresh++;
    if (applyButton) {
      applyButton.disabled = true;
    }
    runButton.disabled = true;
    attention.replaceChildren();
    try {
      // A full run starts from nothing; only a retry of named recipes keeps the others.
      if (!only) {
        await clearFetchRuns();
        hasRuns = false;
        results.replaceChildren();
      }
      for (const recipe of wanted) {
        info(t("Fetching “{recipe}”…", { recipe: recipe.name }));
        const page = pageFor(recipe);
        if (page === null) {
          attention.append(
            h("p", { class: "note error" }, t("“{recipe}” needs an open page of {site}: log in and open it in the browser first.", { recipe: recipe.name, site: recipe.start_site ?? "" })),
          );
          continue;
        }
        try {
          const note = await runOne(recipe, page);
          if (mine !== sequence) {
            return;
          }
          if (note) {
            attention.append(note);
          }
        } catch (error) {
          const reason = String(error);
          attention.append(
            h("p", { class: "note error" }, t("“{recipe}” failed: {reason}", { recipe: recipe.name, reason: reason.replace("[stop] ", "") })),
          );
          if (reason.includes("[stop] ")) {
            // The tab is not where the recipes expect it: the next one would read the wrong page.
            break;
          }
        }
      }
      await showResults();
      info("");
    } finally {
      runButton.disabled = false;
      pageSelect.disabled = false;
      refreshButton.disabled = false;
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
          h("p", { class: "note" }, t("{count} rows were left out.", { count: String(run.skipped.length) })),
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

  loginButton.addEventListener("click", () => {
    loginButton.disabled = true;
    openLoginPages()
      .then(async (result) => {
        await refreshPages();
        const parts: string[] = [];
        if (result.opened.length > 0) {
          parts.push(t("Opened the login page of: {sites}. Log in by hand, then open the page you want.", { sites: result.opened.join(", ") }));
        }
        if (result.already_open.length > 0) {
          parts.push(t("Already open: {sites}.", { sites: result.already_open.join(", ") }));
        }
        if (result.unknown.length > 0) {
          parts.push(t("No login page is known for: {sites}.", { sites: result.unknown.join(", ") }));
        }
        if (parts.length === 0) {
          parts.push(t("No recipe says which site it reads, so there is no login page to open."));
        }
        info(parts.join(" "));
      })
      .catch((error) => fail(String(error)))
      .finally(() => {
        loginButton.disabled = false;
      });
  });
  refreshButton.addEventListener("click", () => void refreshPages());
  runButton.addEventListener("click", () => void runAll());

  function renderList(view: PortfolioView): void {
    if (view.portfolio.recipes.length === 0) {
      list.replaceChildren(
        h("p", { class: "note" }, t("No recipes yet. Fetch a table once by hand in “Import a table”, then use “Save as a recipe”.")),
      );
      runButton.disabled = true;
      return;
    }
    runButton.disabled = false;
    const accountName = (id: number): string =>
      view.portfolio.accounts.find((a) => a.id === id)?.name ?? "?";
    list.replaceChildren(
      ...view.portfolio.recipes.map((recipe) =>
        h(
          "div",
          { class: "add-row" },
          h(
            "span",
            {},
            `${recipe.name} — ${recipe.links.map((l) => l.text).join(" › ") || t("(this page)")} › ${recipe.table_starts_with} → ${accountName(recipe.account)} (${recipe.currency === "" ? t("(the product’s currency)") : recipe.currency})`,
          ),
          deleteButton(() => void ctx.apply({ type: "remove_recipe", name: recipe.name })),
        ),
      ),
    );
  }

  const el = h(
    "section",
    { class: "card recipes" },
    h("h2", {}, t("Fetch recipes")),
    h(
      "p",
      { class: "note" },
      t("After you log in to the browser opened by the app, “Fetch everything” follows each recipe’s links (they only show other pages), reads its table, and shows the result of all of them. Nothing changes until you confirm."),
    ),
    list,
    h("div", { class: "add-row" }, presetSelect, presetAccount, presetAdd),
    noAccountNote,
    presetNote,
    h("div", { class: "add-row" }, loginButton, pageSelect, refreshButton, runButton),
    message,
    attention,
    results,
  );

  void refreshPages();
  void clearFetchRuns();

  return {
    el,
    render(view) {
      current = view;
      renderList(view);
      // Without an account there is nothing to add a preset for: say so instead of leaving the
      // account choice empty.
      noAccountNote.hidden = view.portfolio.accounts.length > 0;
      presetAdd.disabled = view.portfolio.accounts.length === 0;
      const accountsSignature = view.portfolio.accounts.map((a) => `${a.id}:${a.name}`).join("|");
      if (accountsSignature !== presetAccountsSignature) {
        presetAccountsSignature = accountsSignature;
        const previous = presetAccount.value;
        presetAccount.replaceChildren(...view.portfolio.accounts.map((a) => h("option", { value: String(a.id) }, a.name)));
        presetAccount.value = view.portfolio.accounts.some((a) => String(a.id) === previous)
          ? previous
          : String(view.portfolio.accounts[0]?.id ?? "");
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
