import { applyEdit, exportAll, getOverview, getPortfolio } from "./api.ts";
import { h } from "./dom.ts";
import { detectLanguage, getLanguage, LANGUAGES, setLanguage, t, type Lang } from "./i18n.ts";
import type { Context, Panel } from "./panel.ts";
import type { PortfolioView } from "./types.ts";
import { accountsPanel } from "./views/accounts.ts";
import { accountTypesPanel } from "./views/account-types.ts";
import { bitflyerPanel } from "./views/bitflyer.ts";
import { coincheckPanel } from "./views/coincheck.ts";
import { categoriesPanel } from "./views/categories.ts";
import { groupsPanel } from "./views/groups.ts";
import { dataFolderPanel } from "./views/data-folder.ts";
import { importStepsPanel } from "./views/import-steps.ts";
import { accountTabsPanel } from "./views/account-tabs.ts";
import { holdingsTablePanel } from "./views/holdings-table.ts";
import { overviewPanel } from "./views/overview.ts";
import { suggestionsPanel } from "./views/suggestions.ts";
import { HOLDINGS_SUBTABS, pickSubTab, stepSubTab, type HoldingsSubTab } from "./subtabs.ts";
import { dataNotes } from "./views/data-folder.ts";
import { issuesList } from "./views/issues.ts";
import { recordsPanel } from "./views/records.ts";
import { tableImportPanel } from "./views/table-import.ts";
import { recipesPanel } from "./views/recipes.ts";
import { settingsPanel } from "./views/settings.ts";

// The frontend only shows data and sends the user's edits. Every rule, every
// number, and the saving all live in Rust.

const found = document.querySelector<HTMLElement>("#app");
if (!found) {
  throw new Error("missing #app");
}
const app: HTMLElement = found;

let view: PortfolioView | null = null;
let panels: Panel[] = [];
let status = h("p", { class: "status", role: "status" });
let issues = h("div");
let overviewRequest = 0;

type TabName = "holdings" | "import" | "settings";
const TAB_NAMES: TabName[] = ["holdings", "import", "settings"];

function readTab(): TabName {
  try {
    const saved = localStorage.getItem("tab");
    return TAB_NAMES.find((name) => name === saved) ?? "holdings";
  } catch {
    return "holdings";
  }
}

function saveTab(tab: TabName): void {
  try {
    localStorage.setItem("tab", tab);
  } catch {
    // Only a convenience.
  }
}

function readSubTab(): HoldingsSubTab {
  try {
    return pickSubTab(localStorage.getItem("holdings-subtab"));
  } catch {
    return pickSubTab(null);
  }
}

function saveSubTab(tab: HoldingsSubTab): void {
  try {
    localStorage.setItem("holdings-subtab", tab);
  } catch {
    // Only a convenience.
  }
}

function readLanguage(): string | null {
  try {
    return localStorage.getItem("language");
  } catch {
    return null;
  }
}

function saveLanguage(lang: Lang): void {
  try {
    localStorage.setItem("language", lang);
  } catch {
    // The choice is only a convenience; the app works without storage.
  }
}

function showStatus(message: string, isError: boolean): void {
  status.textContent = message;
  status.classList.toggle("error", isError);
  status.setAttribute("role", isError ? "alert" : "status");
}

function renderAll(): void {
  if (!view) {
    return;
  }
  issues.replaceChildren(
    ...[dataNotes(view.data), issuesList(view.issues, view.portfolio)].filter((e) => e !== null),
  );
  for (const panel of panels) {
    panel.render(view);
  }
  // One valuation serves every panel; a newer refresh makes this one obsolete.
  const shown = view;
  const mine = ++overviewRequest;
  getOverview()
    .then((overview) => {
      if (mine === overviewRequest) {
        panels.forEach((panel) => panel.showOverview?.(shown, overview));
      }
    })
    .catch((error) => {
      if (mine === overviewRequest) {
        panels.forEach((panel) => panel.overviewFailed?.(error));
      }
    });
}

// Set by `build()`, which owns the tabs.
let openRecords: () => void = () => {};

let lastError = "";

const context: Context = {
  lastError: () => lastError,
  async apply(edit, at) {
    lastError = "";
    // The edit is tied to the data location of the view it was made from; Rust refuses it if
    // the data folder was switched meanwhile.
    const location = at ?? view?.data.location;
    try {
      if (location === undefined) {
        throw new Error("The data is not loaded yet.");
      }
      view = await applyEdit(edit, location);
      renderAll();
      showStatus(t("Saved."), false);
      return true;
    } catch (error) {
      // The file may have been changed on another computer: look at it again so
      // the controls show what is really saved.
      try {
        view = await getPortfolio();
      } catch {
        // Keep showing the last state.
      }
      lastError = String(error);
      showStatus(t("Could not save: {reason}", { reason: lastError }), true);
      renderAll(); // puts the saved values back into the controls
      return false;
    }
  },
  fail(message) {
    showStatus(message, true);
    renderAll();
  },
  showRecords() {
    openRecords();
  },
  setView(next, message) {
    view = next;
    renderAll();
    showStatus(message, false);
  },
};

function build(): void {
  document.documentElement.lang = getLanguage();
  const language = h(
    "select",
    { "aria-label": t("Language") },
    ...LANGUAGES.map((l) => h("option", { value: l.code }, l.label)),
  );
  language.value = getLanguage();
  language.addEventListener("change", () => {
    const lang = language.value as Lang;
    setLanguage(lang);
    saveLanguage(lang);
    build();
    renderAll();
  });

  const exportButton = h("button", { type: "button" }, t("Export"));
  exportButton.addEventListener("click", () => {
    exportButton.disabled = true;
    showStatus(t("Exporting…"), false);
    exportAll()
      .then((folder) =>
        showStatus(
          t("Exported to {folder}. The files contain your holdings; keep them private.", {
            folder,
          }),
          false,
        ),
      )
      .catch((error) => showStatus(t("Could not export: {reason}", { reason: String(error) }), true))
      .finally(() => {
        exportButton.disabled = false;
      });
  });

  status = h("p", { class: "status", role: "status" });
  issues = h("div");
  const overview = overviewPanel(context);
  const holdingsTable = holdingsTablePanel(context);
  const suggestions = suggestionsPanel(context);
  const accountTabs = accountTabsPanel(context);
  const importSteps = importStepsPanel(context);
  const tableImport = tableImportPanel(context);
  const recipes = recipesPanel(context);
  const bitflyer = bitflyerPanel(context);
  const coincheck = coincheckPanel(context);
  const groups = groupsPanel(context);
  const categories = categoriesPanel(context);
  const records = recordsPanel(context);
  const accounts = accountsPanel(context);
  const accountTypes = accountTypesPanel(context);
  const settings = settingsPanel(context);
  const dataFolder = dataFolderPanel(context);
  panels = [
    overview,
    holdingsTable,
    suggestions,
    accountTabs,
    importSteps,
    tableImport,
    recipes,
    bitflyer,
    coincheck,
    groups,
    categories,
    records,
    accounts,
    accountTypes,
    settings,
    dataFolder,
  ];

  // The wide area beside the balance: the holdings table, the overall suggestion, and the
  // accounts. They only show and hide, like the main tabs; the choice is remembered.
  const subPanes: Record<HoldingsSubTab, HTMLElement> = {
    holdings: h("div", { class: "subpane", role: "tabpanel" }, holdingsTable.el),
    suggestions: h("div", { class: "subpane", role: "tabpanel" }, suggestions.el),
    accounts: h("div", { class: "subpane", role: "tabpanel" }, accountTabs.el),
  };
  const subLabels: Record<HoldingsSubTab, string> = {
    holdings: t("Holdings"),
    suggestions: t("Suggestions"),
    accounts: t("By account"),
  };
  const subButtons = new Map<HoldingsSubTab, HTMLButtonElement>();
  const selectSub = (tab: HoldingsSubTab): void => {
    for (const name of HOLDINGS_SUBTABS) {
      const active = name === tab;
      subPanes[name].hidden = !active;
      const button = subButtons.get(name);
      button?.setAttribute("aria-selected", String(active));
      button?.setAttribute("tabindex", active ? "0" : "-1");
    }
    saveSubTab(tab);
  };
  for (const name of HOLDINGS_SUBTABS) {
    const button = h("button", { type: "button", class: "tab", role: "tab" }, subLabels[name]);
    button.addEventListener("click", () => selectSub(name));
    button.addEventListener("keydown", (event) => {
      const next = stepSubTab(name, event.key);
      if (next !== null) {
        selectSub(next);
        subButtons.get(next)?.focus();
        event.preventDefault();
      }
    });
    subButtons.set(name, button);
  }
  const subTabStrip = h("div", { class: "tabs sub-tabs", role: "tablist", "aria-label": t("Holdings sections") }, ...subButtons.values());
  selectSub(readSubTab());

  // The tabs only show and hide: every panel keeps rendering, so switching loses no input.
  const panes: Record<TabName, HTMLElement> = {
    holdings: h(
      "div",
      { class: "pane holdings-layout", role: "tabpanel", id: "pane-holdings" },
      h("div", { class: "side" }, overview.el),
      h("div", { class: "holdings-main" }, subTabStrip, ...HOLDINGS_SUBTABS.map((n) => subPanes[n])),
    ),
    import: h(
      "div",
      { class: "pane grid", role: "tabpanel", id: "pane-import" },
      importSteps.el,
      recipes.el,
      tableImport.el,
      bitflyer.el,
      coincheck.el,
    ),
    settings: h(
      "div",
      { class: "pane grid", role: "tabpanel", id: "pane-settings" },
      groups.el,
      categories.el,
      accounts.el,
      accountTypes.el,
      records.el,
      settings.el,
      dataFolder.el,
    ),
  };
  const labels: Record<TabName, string> = {
    holdings: t("Holdings"),
    import: t("Import"),
    settings: t("Settings"),
  };
  const buttons = new Map<TabName, HTMLButtonElement>();
  const select = (tab: TabName): void => {
    for (const name of TAB_NAMES) {
      const active = name === tab;
      panes[name].hidden = !active;
      const button = buttons.get(name);
      button?.setAttribute("aria-selected", String(active));
      button?.setAttribute("tabindex", active ? "0" : "-1");
    }
    saveTab(tab);
  };
  for (const name of TAB_NAMES) {
    const button = h("button", { type: "button", class: "tab", role: "tab", "aria-controls": `pane-${name}` }, labels[name]);
    button.addEventListener("click", () => select(name));
    button.addEventListener("keydown", (event) => {
      const step = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0;
      if (step !== 0) {
        const next = TAB_NAMES[(TAB_NAMES.indexOf(name) + step + TAB_NAMES.length) % TAB_NAMES.length] as TabName;
        select(next);
        buttons.get(next)?.focus();
      }
    });
    buttons.set(name, button);
  }
  select(readTab());
  openRecords = () => {
    select("settings");
    records.reveal?.();
  };
  app.replaceChildren(
    h("header", { class: "top" }, h("h1", {}, t("Portfolio")), h("div", { class: "tools" }, exportButton, language)),
    h("div", { class: "tabs main-tabs", role: "tablist", "aria-label": t("Main sections") }, ...buttons.values()),
    status,
    issues,
    h("main", {}, ...TAB_NAMES.map((name) => panes[name])),
  );
}

async function start(): Promise<void> {
  setLanguage(detectLanguage(readLanguage(), navigator.language));
  build();
  showStatus(t("Loading…"), false);
  try {
    view = await getPortfolio();
    showStatus("", false);
    renderAll();
  } catch (error) {
    // An unreadable data file is reported and never overwritten.
    const reason = String(error);
    showStatus(
      reason.includes("schema version")
        ? t("Could not load: {reason}", { reason }) + " " + t("The file is left untouched. To start a new file, move portfolio.json out of the data folder and open the app again.")
        : t("Could not load: {reason}", { reason }),
      true,
    );
  }
}

// Another computer may have saved while this window was in the background: look
// at the file again when the window comes back.
window.addEventListener("focus", () => {
  if (!view) {
    return;
  }
  const asked = view;
  getPortfolio()
    .then((next) => {
      if (view !== asked) {
        return; // an edit (or another refresh) replaced the view while this one was on its way
      }
      if (next.portfolio.revision !== view?.portfolio.revision || JSON.stringify(next.data) !== JSON.stringify(view?.data)) {
        view = next;
        renderAll();
      }
    })
    .catch(() => {
      // Keep showing the last state.
    });
});

void start();
