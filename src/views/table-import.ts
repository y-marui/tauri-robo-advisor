import { clearTable, loadTableFile, loadTableText, planTableImport } from "../api.ts";
import { h } from "../dom.ts";
import { t } from "../i18n.ts";
import type { Context, Panel } from "../panel.ts";
import type { ColumnMapping, PortfolioView, SkipReason, TablePreview } from "../types.ts";
import { importPlanView } from "./import-plan.ts";

// A generic table import: a CSV file or text copied from a brokerage page. The
// user says which column holds the name and which the value; the choice is
// saved per source, so a changed format only needs the columns fixed, not an
// app update. There is no brokerage-specific code. Reading, decoding, and
// picking the columns all happen in Rust.

/** Files larger than this are refused here and again in Rust (2 MiB). */
const MAX_FILE_BYTES = 2 * 1024 * 1024;
const SKIPPED_SHOWN = 8;

function skipLabel(reason: SkipReason): string {
  switch (reason) {
    case "missing_column":
      return t("column missing");
    case "no_name":
      return t("no name");
    case "not_a_number":
      return t("not a number");
    case "out_of_range":
      return t("negative or too large");
  }
}

export function tableImportPanel(ctx: Context): Panel {
  const message = h("p", { class: "note", role: "status" });
  const sourceInput = h("input", {
    type: "text",
    list: "import-sources",
    autocomplete: "off",
    "aria-label": t("Source name"),
    placeholder: t("Source name"),
  });
  const sourceList = h("datalist", { id: "import-sources" });
  const currencySelect = h("select", { "aria-label": t("Currency of the values") });
  const fileInput = h("input", {
    type: "file",
    accept: ".csv,.tsv,.txt,text/csv,text/plain",
    "aria-label": t("Choose a file"),
  });
  const pasteArea = h("textarea", {
    rows: 4,
    "aria-label": t("Pasted table"),
    placeholder: t("Or paste the table here"),
  });
  const pasteButton = h("button", { type: "button" }, t("Read pasted text"));

  const previewBox = h("div", { class: "table-preview" });
  const nameSelect = h("select", { "aria-label": t("Name column") });
  const valueSelect = h("select", { "aria-label": t("Value column") });
  const headerInput = h("input", {
    type: "number",
    min: 0,
    value: 0,
    class: "number",
    "aria-label": t("Heading rows"),
  });
  const saveMapping = h("button", { type: "button" }, t("Save columns"));
  const planButton = h("button", { type: "button" }, t("Preview import"));
  const accountSelect = h("select", { "aria-label": t("Account to update") });
  const mappingBox = h(
    "div",
    { class: "mapping", hidden: true },
    h("div", { class: "fields" },
      h("label", { class: "field" }, h("span", {}, t("Name column")), nameSelect),
      h("label", { class: "field" }, h("span", {}, t("Value column")), valueSelect),
      h("label", { class: "field" }, h("span", {}, t("Heading rows")), headerInput),
      h("label", { class: "field" }, h("span", {}, t("Account to update")), accountSelect),
    ),
    h("div", { class: "add-row" }, saveMapping, planButton),
  );
  const result = h("div", { class: "import-result", "aria-live": "polite" });

  let current: PortfolioView | null = null;
  let preview: TablePreview | null = null;
  let planned = false;
  let sequence = 0;
  let accountSignature: string | null = null;
  let sourceSignature: string | null = null;

  const fail = (text: string): void => {
    message.classList.add("error");
    message.textContent = text;
  };
  const info = (text: string): void => {
    message.classList.remove("error");
    message.textContent = text;
  };

  const source = (): string => sourceInput.value.trim();
  const mapping = (): ColumnMapping | null => {
    const header = Number(headerInput.value);
    if (nameSelect.value === "" || valueSelect.value === "" || !Number.isInteger(header) || header < 0) {
      return null;
    }
    return { name: Number(nameSelect.value), value: Number(valueSelect.value), header_rows: header };
  };

  /** Fills the column selects from the preview and the saved mapping of the source. */
  function useSavedMapping(): void {
    const saved = current?.portfolio.import_mappings.find((m) => m.source === source());
    if (!saved || !preview) {
      return;
    }
    if (saved.name_column < preview.columns && saved.value_column < preview.columns) {
      nameSelect.value = String(saved.name_column);
      valueSelect.value = String(saved.value_column);
      headerInput.value = String(saved.header_rows);
    }
  }

  function showPreview(loaded: TablePreview): void {
    preview = loaded;
    planned = false;
    // A preview still in flight belongs to the table that is being replaced.
    sequence += 1;
    result.replaceChildren();
    const columns = Array.from({ length: loaded.columns }, (_, i) => i);
    const label = (i: number): string => {
      const first = loaded.rows[0]?.[i] ?? "";
      return first === "" ? `${t("Column")} ${i + 1}` : `${t("Column")} ${i + 1}: ${first}`;
    };
    for (const select of [nameSelect, valueSelect]) {
      select.replaceChildren(...columns.map((i) => h("option", { value: String(i) }, label(i))));
    }
    nameSelect.value = "0";
    valueSelect.value = String(Math.min(1, Math.max(loaded.columns - 1, 0)));
    headerInput.value = "1";
    useSavedMapping();

    previewBox.replaceChildren(
      h(
        "p",
        { class: "muted" },
        t("{rows} rows, {columns} columns", {
          rows: String(loaded.total_rows),
          columns: String(loaded.columns),
        }),
      ),
      h(
        "div",
        { class: "table-wrap" },
        h(
          "table",
          {},
          h("thead", {}, h("tr", {}, ...columns.map((i) => h("th", {}, `${t("Column")} ${i + 1}`)))),
          h(
            "tbody",
            {},
            ...loaded.rows.map((row) =>
              h("tr", {}, ...columns.map((i) => h("td", {}, row[i] ?? ""))),
            ),
          ),
        ),
      ),
    );
    mappingBox.hidden = false;
    info("");
  }

  async function load(read: () => Promise<TablePreview>): Promise<void> {
    info(t("Reading…"));
    try {
      showPreview(await read());
    } catch (error) {
      preview = null;
      planned = false;
      sequence += 1;
      mappingBox.hidden = true;
      previewBox.replaceChildren();
      result.replaceChildren();
      fail(t("Could not read: {reason}", { reason: String(error) }));
    }
  }

  fileInput.addEventListener("change", () => {
    const file = fileInput.files?.[0];
    if (!file) {
      return;
    }
    if (file.size > MAX_FILE_BYTES) {
      fail(t("The file is too large."));
      return;
    }
    void load(async () => loadTableFile(Array.from(new Uint8Array(await file.arrayBuffer()))));
  });

  pasteButton.addEventListener("click", () => {
    const text = pasteArea.value;
    if (text.trim() === "") {
      fail(t("Paste a table first."));
      return;
    }
    void load(() => loadTableText(text));
  });

  sourceInput.addEventListener("change", () => {
    useSavedMapping();
    planned = false;
    sequence += 1;
    result.replaceChildren();
  });

  saveMapping.addEventListener("click", () => {
    const chosen = mapping();
    if (!chosen || source() === "") {
      fail(t("Enter a source name and choose the columns first."));
      return;
    }
    void ctx.apply({
      type: "set_import_mapping",
      source: source(),
      name_column: chosen.name,
      value_column: chosen.value,
      header_rows: chosen.header_rows,
    });
  });

  async function showPlan(): Promise<void> {
    const view = current;
    const chosen = mapping();
    if (!view || !preview || !chosen || source() === "") {
      fail(t("Enter a source name and choose the columns first."));
      return;
    }
    const mine = ++sequence;
    const planResult = await planTableImport(source(), currencySelect.value, chosen);
    if (mine !== sequence) {
      return;
    }
    planned = true;
    const notes: HTMLElement[] = [];
    if (planResult.skipped.length > 0) {
      const shown = planResult.skipped
        .slice(0, SKIPPED_SHOWN)
        .map((s) => `${t("row")} ${s.row} (${skipLabel(s.reason)})`)
        .join(", ");
      notes.push(
        h(
          "p",
          { class: "note error" },
          t("{count} rows were left out: {rows}", {
            count: String(planResult.skipped.length),
            rows: planResult.skipped.length > SKIPPED_SHOWN ? `${shown}, …` : shown,
          }),
        ),
      );
    }
    result.replaceChildren(
      h("p", { class: "muted" }, t("{count} rows read", { count: String(planResult.rows_read) })),
      importPlanView({
        ctx,
        view,
        plan: planResult.plan,
        source: source(),
        currency: currencySelect.value,
        accountSelect,
        notes,
      }),
    );
  }

  const run = (): void => {
    showPlan().catch((error) => fail(t("Could not read: {reason}", { reason: String(error) })));
  };
  planButton.addEventListener("click", run);
  accountSelect.addEventListener("change", () => {
    if (planned) {
      run();
    }
  });

  const el = h(
    "section",
    { class: "card table-import" },
    h("h2", {}, t("Import a table")),
    h(
      "p",
      { class: "note" },
      t("Log in to your brokerage in your own browser, then save the holdings as a CSV file or copy the table, and load it here."),
    ),
    h(
      "div",
      { class: "fields" },
      h("label", { class: "field" }, h("span", {}, t("Source name")), sourceInput, sourceList),
      h("label", { class: "field" }, h("span", {}, t("Currency of the values")), currencySelect),
    ),
    h("div", { class: "add-row" }, fileInput),
    pasteArea,
    h("div", { class: "add-row" }, pasteButton),
    message,
    previewBox,
    mappingBox,
    result,
  );

  void clearTable();

  return {
    el,
    render(view) {
      current = view;

      const accounts = `${view.portfolio.accounts.map((a) => `${a.id}:${a.name}`).join(",")}`;
      if (accounts !== accountSignature) {
        accountSignature = accounts;
        const previous = accountSelect.value;
        accountSelect.replaceChildren(
          ...view.portfolio.accounts.map((a) => h("option", { value: String(a.id) }, a.name)),
        );
        if (view.portfolio.accounts.some((a) => String(a.id) === previous)) {
          accountSelect.value = previous;
        }
      }

      const known = new Set<string>(view.portfolio.import_mappings.map((m) => m.source));
      for (const product of view.portfolio.products) {
        for (const alias of product.aliases) {
          known.add(alias.source);
        }
      }
      const currencies = new Set<string>(["JPY", "USD"]);
      view.portfolio.products.forEach((p) => currencies.add(p.currency));
      const signature = `${[...known].sort().join("|")}#${[...currencies].sort().join("|")}`;
      if (signature !== sourceSignature) {
        sourceSignature = signature;
        sourceList.replaceChildren(...[...known].sort().map((s) => h("option", { value: s })));
        const previous = currencySelect.value;
        currencySelect.replaceChildren(
          ...[...currencies].sort().map((c) => h("option", { value: c }, c)),
        );
        currencySelect.value = currencies.has(previous) ? previous : "JPY";
      }

      // An alias was linked or unlinked: show the new result.
      if (planned) {
        run();
      }
    },
  };
}
