import {
  browserPages,
  closeBrowser,
  fetchBrowserTables,
  listBrowsers,
  openBrowser,
  useFetchedTable,
} from "../api.ts";
import { h } from "../dom.ts";
import { t } from "../i18n.ts";
import type { BrowserKind, BrowserPage, CardSource, FetchedTable, TablePreview } from "../types.ts";

// An optional way to get a table: the app opens Chrome, Edge, or Brave with a
// profile of its own, the user logs in and opens the holdings page by hand,
// and the app reads the tables on that page. Nothing is typed or clicked for
// the user, and no credential passes through the app. The table then goes
// through the same column choice as a file or pasted text.

const PROFILE_KEY = "browserProfile";

function remembered(): string {
  try {
    return localStorage.getItem(PROFILE_KEY) ?? "";
  } catch {
    return "";
  }
}

function remember(folder: string): void {
  try {
    localStorage.setItem(PROFILE_KEY, folder);
  } catch {
    // Only a convenience; the app works without storage.
  }
}

function kindLabel(kind: BrowserKind): string {
  switch (kind) {
    case "chrome":
      return "Chrome";
    case "edge":
      return "Edge";
    case "brave":
      return "Brave";
  }
}

interface Options {
  /** Receives the preview of the table the user chose, and the text its first cell starts with. */
  onTable(preview: TablePreview, label: string, card: CardSource | null): void;
  info(text: string): void;
  fail(text: string): void;
}

export function browserFetch(options: Options): HTMLElement {
  const { onTable, info, fail } = options;
  const browserSelect = h("select", { "aria-label": t("Browser") });
  const profileInput = h("input", {
    type: "text",
    autocomplete: "off",
    "aria-label": t("Profile folder"),
    placeholder: t("Profile folder (empty: inside the app's data folder)"),
    value: remembered(),
  });
  const openButton = h("button", { type: "button" }, t("Open browser"));
  const pageSelect = h("select", { "aria-label": t("Page") });
  const refreshButton = h("button", { type: "button" }, t("Refresh pages"));
  const fetchButton = h("button", { type: "button" }, t("Fetch tables"));
  const tableSelect = h("select", { "aria-label": t("Table") });
  const useButton = h("button", { type: "button" }, t("Use this table"));
  const closeButton = h("button", { type: "button" }, t("Close browser"));
  let lastTables: FetchedTable[] = [];
  const afterOpen = h(
    "div",
    { hidden: true },
    h("div", { class: "add-row" }, pageSelect, refreshButton, fetchButton),
    h("div", { class: "add-row", id: "fetched-tables", hidden: true }, tableSelect, useButton),
    h("div", { class: "add-row" }, closeButton),
  );
  const fetchedRow = afterOpen.querySelector<HTMLElement>("#fetched-tables");
  const controls = h(
    "div",
    { hidden: true },
    h("div", { class: "add-row" }, browserSelect, profileInput, openButton),
    afterOpen,
  );
  const none = h("p", { class: "note" }, t("No Chrome, Edge, or Brave was found on this computer."));
  none.hidden = true;

  const run = async (button: HTMLButtonElement, work: () => Promise<void>): Promise<void> => {
    button.disabled = true;
    try {
      await work();
    } catch (error) {
      fail(String(error));
    } finally {
      button.disabled = false;
    }
  };

  async function refreshPages(): Promise<BrowserPage[]> {
    const pages = await browserPages();
    pageSelect.replaceChildren(
      ...pages.map((p) => h("option", { value: p.id }, `${p.host} — ${p.title || t("(no title)")}`)),
    );
    if (pages.length === 0) {
      info(t("No web page is open in the browser yet."));
    }
    return pages;
  }

  openButton.addEventListener("click", () => {
    void run(openButton, async () => {
      info(t("Opening the browser…"));
      const folder = profileInput.value.trim();
      remember(folder);
      await openBrowser(browserSelect.value as BrowserKind, folder === "" ? null : folder);
      afterOpen.hidden = false;
      await refreshPages();
      info(t("Log in to your brokerage in that browser, open the holdings page, then fetch the tables."));
    });
  });

  refreshButton.addEventListener("click", () => {
    void run(refreshButton, async () => {
      await refreshPages();
    });
  });

  fetchButton.addEventListener("click", () => {
    void run(fetchButton, async () => {
      if (pageSelect.value === "") {
        fail(t("Open the holdings page in the browser first."));
        return;
      }
      info(t("Reading the page…"));
      const fetched = await fetchBrowserTables(pageSelect.value);
      const tables: FetchedTable[] = fetched.tables;
      lastTables = tables;
      tableSelect.replaceChildren(
        ...tables.map((table) =>
          h(
            "option",
            { value: String(table.index) },
            t(table.card ? "{rows} cards × {columns} lines: {label}" : "{rows} rows × {columns} columns: {label}", {
              rows: String(table.rows),
              columns: String(table.columns),
              label: table.label,
            }),
          ),
        ),
      );
      if (fetchedRow) {
        fetchedRow.hidden = tables.length === 0;
      }
      info(
        tables.length === 0
          ? t("No table was found on that page.")
          : fetched.headings_cut
            ? t("The page has more headings than the app reads, so card lists below the first ones are not offered. Choose the table with the holdings, then use it.")
            : t("Choose the table with the holdings, then use it."),
      );
    });
  });

  useButton.addEventListener("click", () => {
    void run(useButton, async () => {
      if (tableSelect.value === "") {
        return;
      }
      const index = Number(tableSelect.value);
      const chosen = lastTables.find((table) => table.index === index);
      onTable(await useFetchedTable(index), chosen?.label ?? "", chosen?.card ?? null);
    });
  });

  closeButton.addEventListener("click", () => {
    void run(closeButton, async () => {
      await closeBrowser();
      afterOpen.hidden = true;
      if (fetchedRow) {
        fetchedRow.hidden = true;
      }
      info(t("The browser was closed."));
    });
  });

  listBrowsers()
    .then((found) => {
      browserSelect.replaceChildren(
        ...found.map((b) => h("option", { value: b.kind }, kindLabel(b.kind))),
      );
      controls.hidden = found.length === 0;
      none.hidden = found.length !== 0;
    })
    .catch((error) => fail(String(error)));

  return h(
    "details",
    { class: "browser-fetch" },
    h("summary", {}, t("Fetch with the browser (optional)")),
    h(
      "p",
      { class: "note" },
      t("Opens Chrome, Edge, or Brave with a profile of its own. You log in yourself; the app only reads the tables on the page you open."),
    ),
    none,
    controls,
  );
}
