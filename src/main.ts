import { applyEdit, getPortfolio } from "./api.ts";
import { h } from "./dom.ts";
import { detectLanguage, getLanguage, LANGUAGES, setLanguage, t, type Lang } from "./i18n.ts";
import type { Context, Panel } from "./panel.ts";
import type { PortfolioView } from "./types.ts";
import { accountsPanel } from "./views/accounts.ts";
import { categoriesPanel } from "./views/categories.ts";
import { holdingsPanel } from "./views/holdings.ts";
import { issuesList } from "./views/issues.ts";
import { productsPanel } from "./views/products.ts";
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
  issues.replaceChildren(...[issuesList(view.issues, view.portfolio)].filter((e) => e !== null));
  for (const panel of panels) {
    panel.render(view);
  }
}

const context: Context = {
  async apply(edit) {
    try {
      view = await applyEdit(edit);
      renderAll();
      showStatus(t("Saved."), false);
      return true;
    } catch (error) {
      showStatus(t("Could not save: {reason}", { reason: String(error) }), true);
      renderAll(); // puts the previous values back into the controls
      return false;
    }
  },
  fail(message) {
    showStatus(message, true);
    renderAll();
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

  status = h("p", { class: "status", role: "status" });
  issues = h("div");
  panels = [
    categoriesPanel(context),
    productsPanel(context),
    accountsPanel(context),
    holdingsPanel(context),
    settingsPanel(context),
  ];
  app.replaceChildren(
    h("header", { class: "top" }, h("h1", {}, t("Portfolio")), language),
    status,
    issues,
    h("main", { class: "grid" }, ...panels.map((p) => p.el)),
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

void start();
