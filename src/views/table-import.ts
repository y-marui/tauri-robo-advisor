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
  const modeSelect = h(
    "select",
    { "aria-label": t("The value is") },
    h("option", { value: "column" }, t("A column")),
    h("option", { value: "product" }, t("Quantity × price")),
  );
  const priceSelect = h("select", { "aria-label": t("Price column") });
  const unitSelect = h(
    "select",
    { "aria-label": t("Price is quoted per") },
    h("option", { value: "1" }, t("1 unit")),
    h("option", { value: "10000" }, t("10,000 units")),
  );
  const perRecordSelect = h(
    "select",
    { "aria-label": t("Rows per record") },
    ...[1, 2, 3, 4].map((n) => h("option", { value: String(n) }, String(n))),
  );
  const nameRowSelect = h("select", { "aria-label": t("Row with the name") });
  const valueRowSelect = h("select", { "aria-label": t("Row with the numbers") });
  const saveMapping = h("button", { type: "button" }, t("Save columns"));
  const planButton = h("button", { type: "button" }, t("Preview import"));
  const accountSelect = h("select", { "aria-label": t("Account to update") });
  const valueLabel = h("span", {}, "");
  const mappingBox = h(
    "div",
    { class: "mapping", hidden: true },
    h("div", { class: "fields" },
      h("label", { class: "field" }, h("span", {}, t("Name column")), nameSelect),
      h("label", { class: "field" }, h("span", {}, t("The value is")), modeSelect),
      h("label", { class: "field" }, valueLabel, valueSelect),
      h("label", { class: "field price-only" }, h("span", {}, t("Price column")), priceSelect),
      h("label", { class: "field price-only" }, h("span", {}, t("Price is quoted per")), unitSelect),
      h("label", { class: "field" }, h("span", {}, t("Heading rows")), headerInput),
      h("label", { class: "field" }, h("span", {}, t("Rows per record")), perRecordSelect),
      h("label", { class: "field" }, h("span", {}, t("Row with the name")), nameRowSelect),
      h("label", { class: "field" }, h("span", {}, t("Row with the numbers")), valueRowSelect),
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
    const product = modeSelect.value === "product";
    if (
      nameSelect.value === "" ||
      valueSelect.value === "" ||
      (product && priceSelect.value === "") ||
      !Number.isInteger(header) ||
      header < 0
    ) {
      return null;
    }
    return {
      name: Number(nameSelect.value),
      value: Number(valueSelect.value),
      price: product ? Number(priceSelect.value) : null,
      price_unit: product ? Number(unitSelect.value) : 1,
      header_rows: header,
      rows_per_record: Number(perRecordSelect.value),
      name_row: Number(nameRowSelect.value),
      value_row: Number(valueRowSelect.value),
    };
  };

  /** Shows the controls that fit the mode, and the rows a record can have. */
  function syncControls(): void {
    const product = modeSelect.value === "product";
    valueLabel.textContent = product ? t("Quantity column") : t("Value column");
    for (const field of mappingBox.querySelectorAll<HTMLElement>(".price-only")) {
      field.hidden = !product;
    }
    const perRecord = Number(perRecordSelect.value);
    for (const select of [nameRowSelect, valueRowSelect]) {
      const previous = select.value;
      select.replaceChildren(
        ...Array.from({ length: perRecord }, (_, i) => h("option", { value: String(i) }, String(i + 1))),
      );
      select.value = Number(previous) < perRecord && previous !== "" ? previous : "0";
    }
  }
  modeSelect.addEventListener("change", syncControls);
  perRecordSelect.addEventListener("change", syncControls);

  /** Fills the column selects from the preview and the saved mapping of the source. */
  function useSavedMapping(): void {
    const saved = current?.portfolio.import_mappings.find((m) => m.source === source());
    const loaded = preview;
    if (!saved || !loaded) {
      return;
    }
    const columns = [saved.name_column, saved.value_column, saved.price_column ?? 0];
    if (columns.every((c) => c < loaded.columns)) {
      nameSelect.value = String(saved.name_column);
      valueSelect.value = String(saved.value_column);
      modeSelect.value = saved.price_column === null ? "column" : "product";
      priceSelect.value = String(saved.price_column ?? Math.min(2, loaded.columns - 1));
      unitSelect.value = String(saved.price_unit);
      headerInput.value = String(saved.header_rows);
      perRecordSelect.value = String(saved.rows_per_record);
      syncControls();
      nameRowSelect.value = String(saved.name_row);
      valueRowSelect.value = String(saved.value_row);
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
    for (const select of [nameSelect, valueSelect, priceSelect]) {
      select.replaceChildren(...columns.map((i) => h("option", { value: String(i) }, label(i))));
    }
    nameSelect.value = "0";
    valueSelect.value = String(Math.min(1, Math.max(loaded.columns - 1, 0)));
    priceSelect.value = String(Math.min(2, Math.max(loaded.columns - 1, 0)));
    modeSelect.value = "column";
    unitSelect.value = "1";
    headerInput.value = "1";
    perRecordSelect.value = "1";
    syncControls();
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
      price_column: chosen.price,
      price_unit: chosen.price_unit,
      header_rows: chosen.header_rows,
      rows_per_record: chosen.rows_per_record,
      name_row: chosen.name_row,
      value_row: chosen.value_row,
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
