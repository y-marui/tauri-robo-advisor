import { applyEdit, exportAll, getOverview, getPortfolio } from "./api.ts";
import { h } from "./dom.ts";
import { detectLanguage, getLanguage, LANGUAGES, setLanguage, t, type Lang } from "./i18n.ts";
import type { Context, Panel } from "./panel.ts";
import type { PortfolioView } from "./types.ts";
import { accountsPanel } from "./views/accounts.ts";
import { bitflyerPanel } from "./views/bitflyer.ts";
import { categoriesPanel } from "./views/categories.ts";
import { dataFolderPanel } from "./views/data-folder.ts";
import { holdingsPanel } from "./views/holdings.ts";
import { accountTabsPanel } from "./views/account-tabs.ts";
import { holdingsTablePanel } from "./views/holdings-table.ts";
import { overviewPanel } from "./views/overview.ts";
import { dataNotes } from "./views/data-folder.ts";
import { issuesList } from "./views/issues.ts";
import { productsPanel } from "./views/products.ts";
import { tableImportPanel } from "./views/table-import.ts";
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
const DASHBOARD_PANELS = 3;

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

const context: Context = {
  async apply(edit) {
    try {
      view = await applyEdit(edit);
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
      showStatus(t("Could not save: {reason}", { reason: String(error) }), true);
      renderAll(); // puts the saved values back into the controls
      return false;
    }
  },
  fail(message) {
    showStatus(message, true);
    renderAll();
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
  // The three dashboard panels come first (left, center, right), then the
  // manual entry cards.
  panels = [
    overviewPanel(context),
    holdingsTablePanel(context),
    accountTabsPanel(context),
    tableImportPanel(context),
    bitflyerPanel(context),
    categoriesPanel(context),
    productsPanel(context),
    accountsPanel(context),
    holdingsPanel(context),
    settingsPanel(context),
    dataFolderPanel(context),
  ];
  app.replaceChildren(
    h("header", { class: "top" }, h("h1", {}, t("Portfolio")), h("div", { class: "tools" }, exportButton, language)),
    status,
    issues,
    h("main", { class: "dashboard" }, ...panels.slice(0, DASHBOARD_PANELS).map((p) => p.el)),
    h("main", { class: "grid" }, ...panels.slice(DASHBOARD_PANELS).map((p) => p.el)),
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
    showStatus(t("Could not load: {reason}", { reason: String(error) }), true);
  }
}

// Another computer may have saved while this window was in the background: look
// at the file again when the window comes back.
window.addEventListener("focus", () => {
  if (!view) {
    return;
  }
  getPortfolio()
    .then((next) => {
      if (next.portfolio.revision !== view?.portfolio.revision || next.data.folder !== view?.data.folder) {
        view = next;
        renderAll();
      }
    })
    .catch(() => {
      // Keep showing the last state.
    });
});

void start();
