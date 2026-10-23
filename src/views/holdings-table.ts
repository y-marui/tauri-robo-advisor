import { formatAmount, fractionToPercent } from "../decimal.ts";
import { h } from "../dom.ts";
import { getLanguage, t } from "../i18n.ts";
import { panel, type Context, type Panel } from "../panel.ts";
import type { Category, Holding, Overview, PortfolioView } from "../types.ts";
import { groupEntries, groupName } from "./groups.ts";
import { matchesTypeFilter, NO_TYPE, recordKey, typeFilterKey, typeLabel } from "./labels.ts";

// The center panel: holding records grouped by category (or by account), each group opened by
// a subtotal row. The table only shows: names, categories and amounts (in the product's own
// currency) are edited in the Records card of the Settings tab. The JPY values and shares
// come from Rust.

const percent = (fraction: string): string => `${fractionToPercent(fraction)}%`;
const yen = (amount: string): string => formatAmount(amount, getLanguage());

/** The record's value rounded by Rust for display; the exact value if Rust gave none. */
const shownValue = (view: PortfolioView, record: Holding): string =>
  view.shown_values.find(
    (s) =>
      s.account === record.account &&
      s.product === record.product &&
      (s.account_type ?? null) === (record.account_type ?? null),
  )?.value ?? record.value;

export function holdingsTablePanel(ctx: Context): Panel {
  // Cells that show Rust's valuation, found again by key after each refresh.
  let holdingCells = new Map<string, { value: HTMLElement; share: HTMLElement }>();
  // The rows of each category in "By category", so they can be ordered by value once the valuation arrives.
  let categoryBlocks: { header: HTMLElement; records: { key: string; product: number; account: number; row: HTMLElement }[] }[] = [];
  let accountCells = new Map<number, { value: HTMLElement; share: HTMLElement }>();
  let categoryCells = new Map<number, { value: HTMLElement; share: HTMLElement }>();
  // The group subtotals; the key is the group id, or "none" for the ungrouped categories.
  let groupCells = new Map<string, { value: HTMLElement; share: HTMLElement }>();
  const note = h("p", { class: "note", role: "status" });
  const inFilter = (record: Holding): boolean => matchesTypeFilter(record.account_type, typeFilter);
  let groupBy: "category" | "account" | "type" = "category";
  // Show only the records of one type: a type id, "none" for the records without a type, "" for all.
  let typeFilter = "";
  let typeCells = new Map<string, { value: HTMLElement; share: HTMLElement }>();
  // The total of the whole portfolio, the first row of the table.
  let totalCell = h("td", { class: "num" });
  let lastView: PortfolioView | null = null;
  let lastOverview: Overview | null = null;

  const editButton = (): HTMLButtonElement => {
    const button = h("button", { type: "button", class: "link-button" }, t("Edit in Records"));
    button.addEventListener("click", () => ctx.showRecords());
    return button;
  };

  const table = panel(
    "Holdings",
    (view) =>
      [
        groupBy,
        typeFilter,
        (view.portfolio.custom_types ?? []).map((c) => `${c.id}:${c.name}`).join(","),
        view.portfolio.accounts.map((a) => `${a.id}:${a.name}`).join(","),
        view.portfolio.categories.map((c) => `${c.id}:${c.name}:${c.group ?? ""}`).join(","),
        view.portfolio.groups.map((g) => `${g.id}:${g.name}`).join(","),
        // Everything shown as text is part of the signature, so an edit in Settings rebuilds the table.
        view.portfolio.products.map((p) => `${p.id}:${p.name}:${p.category}:${p.currency}`).join(","),
        view.portfolio.holdings.map((x) => `${recordKey(x.account, x.product, x.account_type)}:${x.value}`).join(","),
      ].join("|"),
    (view, bind) => {
      holdingCells = new Map();
      categoryBlocks = [];
      categoryCells = new Map();
      groupCells = new Map();
      accountCells = new Map();
      typeCells = new Map();
      const { categories, products, accounts } = view.portfolio;
      if (view.portfolio.holdings.length === 0) {
        return [h("p", { class: "note" }, t("No holdings yet.")), editButton()];
      }
      // A subtotal is the figure of the whole portfolio, so with a filter on the subtotals are
      // left empty (the rows below them are only some of the records).
      const holdings = view.portfolio.holdings.filter(inFilter);

      let current: PortfolioView = view;
      // The quantity and price cells follow every refresh (an import or an edit changes them
      // without changing which holdings there are).
      const refreshers: (() => void)[] = [];
      bind((v) => {
        current = v;
        for (const refresh of refreshers) {
          refresh();
        }
      });
      const holdingOf = (v: PortfolioView, record: Holding) =>
        v.portfolio.holdings.find(
          (x) =>
            x.account === record.account &&
            x.product === record.product &&
            (x.account_type ?? null) === (record.account_type ?? null),
        );

      const rows: HTMLElement[] = [];
      totalCell = h("td", { class: "num" });
      rows.push(
        h(
          "tr",
          { class: "group group-main" },
          h("th", { scope: "rowgroup", colspan: 4 }, t("Total")),
          h("td", {}),
          h("td", {}),
          h("td", {}),
          totalCell,
          h("td", {}),
        ),
      );

      const recordRow = (record: Holding): HTMLElement | null => {
        const product = products.find((p) => p.id === record.product);
        const account = accounts.find((a) => a.id === record.account);
        if (!product || !account) {
          return null;
        }
        const cells = { value: h("td", { class: "num" }), share: h("td", { class: "num" }) };
        holdingCells.set(recordKey(record.account, record.product, record.account_type), cells);
        const quantityCell = h("td", { class: "num" });
        const priceCell = h("td", { class: "num" });
        const showQuantityAndPrice = (): void => {
          const held = holdingOf(current, record);
          quantityCell.textContent = held?.quantity ?? "";
          const price = priceText(held);
          priceCell.textContent = price === "" ? "" : `${price} ${product.currency}`;
        };
        showQuantityAndPrice();
        refreshers.push(showQuantityAndPrice);
        const row = h(
          "tr",
          {},
            h(
              "th",
              { scope: "row", class: "product-cell", title: product.name },
              product.name,
            ),
            ...(groupBy === "category" ? [h("td", { class: "clip" }, account.name)] : []),
            h(
              "td",
              { class: "clip record-type" },
              record.account_type ? typeLabel(record.account_type, view.portfolio.custom_types) : "",
            ),
            h(
              "td",
              { class: "clip" },
              categories.find((c) => c.id === product.category)?.name ?? "",
            ),
            priceCell,
            quantityCell,
            h(
              "td",
              { class: "num" },
              `${yen(shownValue(view, record))} ${product.currency}`,
            ),
            cells.value,
            cells.share,
        );
        rows.push(row);
        return row;
      };

      if (groupBy === "account") {
        for (const account of accounts) {
          const records = holdings.filter((x) => x.account === account.id);
          if (records.length === 0) {
            continue;
          }
          const subtotal = { value: h("td", { class: "num" }), share: h("td", { class: "num" }) };
          accountCells.set(account.id, subtotal);
          rows.push(
            h(
              "tr",
              { class: "group" },
              h("th", { scope: "rowgroup", colspan: 4 }, account.name),
              h("td", {}),
              h("td", {}),
              h("td", {}),
              subtotal.value,
              subtotal.share,
            ),
          );
          for (const record of records) {
            recordRow(record);
          }
        }
      } else if (groupBy === "type") {
        const ids = [...new Set(holdings.map((x) => x.account_type ?? null))].sort((a, b) =>
          typeLabel(a, view.portfolio.custom_types).localeCompare(typeLabel(b, view.portfolio.custom_types)),
        );
        for (const id of ids) {
          const records = holdings.filter((x) => (x.account_type ?? null) === id);
          const subtotal = { value: h("td", { class: "num" }), share: h("td", { class: "num" }) };
          typeCells.set(typeFilterKey(id), subtotal);
          rows.push(
            h(
              "tr",
              { class: "group" },
              h("th", { scope: "rowgroup", colspan: 4 }, typeLabel(id, view.portfolio.custom_types)),
              h("td", {}),
              h("td", {}),
              h("td", {}),
              subtotal.value,
              subtotal.share,
            ),
          );
          for (const record of records) {
            recordRow(record);
          }
        }
      } else {
        const categoryRows = (category: Category): void => {
          const own = products.filter((p) => p.category === category.id);
          const records = holdings.filter((x) => own.some((p) => p.id === x.product));

          const subtotal = { value: h("td", { class: "num" }), share: h("td", { class: "num" }) };
          categoryCells.set(category.id, subtotal);
          rows.push(
            h(
              "tr",
              { class: "group" },
              h(
                "th",
                { scope: "rowgroup", colspan: 4 },
                category.name,
              ),
              h("td", {}),
              h("td", {}),
              h("td", {}),
              subtotal.value,
              subtotal.share,
            ),
          );
          const block: (typeof categoryBlocks)[number] = { header: rows[rows.length - 1] as HTMLElement, records: [] };
          for (const record of records) {
            const row = recordRow(record);
            if (row) {
              block.records.push({
                key: recordKey(record.account, record.product, record.account_type),
                product: record.product,
                account: record.account,
                row,
              });
            }
          }
          categoryBlocks.push(block);
        };
        const entries = groupEntries(view);
        const groupRow = (id: number | null): void => {
          const subtotal = { value: h("td", { class: "num" }), share: h("td", { class: "num" }) };
          groupCells.set(id === null ? "none" : String(id), subtotal);
          rows.push(
            h(
              "tr",
              { class: "group group-main" },
              h(
                "th",
                { scope: "rowgroup", colspan: 4 },
                groupName(view, id),
              ),
              h("td", {}),
              h("td", {}),
              h("td", {}),
              subtotal.value,
              subtotal.share,
            ),
          );
        };
        if (entries.length === 0) {
          for (const category of categories) {
            categoryRows(category);
          }
        } else {
          for (const entry of entries) {
            groupRow(entry.id);
            for (const category of entry.categories) {
              categoryRows(category);
            }
          }
        }
      }

      return [
        h(
          "div",
          { class: "table-wrap" },
          h(
            "table",
            {},
            h(
              "thead",
              {},
              h(
                "tr",
                {},
                h("th", {}, t("Product")),
                ...(groupBy === "category" ? [h("th", {}, t("Account"))] : []),
                h("th", {}, t("Type")),
                h("th", {}, t("Category")),
                h("th", {}, t("Price")),
                h("th", {}, t("Quantity")),
                h("th", {}, t("Value")),
                h("th", {}, t("Value (JPY)")),
                h("th", {}, t("Share")),
              ),
            ),
            h("tbody", {}, ...rows),
          ),
        ),
      ];
    },
    { heading: false },
  );
  table.el.append(note);

  const switcher = h(
    "select",
    { "aria-label": t("Group by") },
    h("option", { value: "category" }, t("By category")),
    h("option", { value: "account" }, t("By account")),
    h("option", { value: "type" }, t("By type")),
  );
  const filter = h("select", { "aria-label": t("Show type") });
  const refill = (view: PortfolioView): void => {
    const present = [...new Set(view.portfolio.holdings.map((x) => typeFilterKey(x.account_type)))];
    const options = [
      { value: "", label: t("All types") },
      ...present.map((key) => ({
        value: key,
        label: typeLabel(key === NO_TYPE ? null : key, view.portfolio.custom_types),
      })),
    ];
    // A type that no record has any more cannot stay selected.
    if (typeFilter !== "" && !present.includes(typeFilter)) {
      typeFilter = "";
    }
    filter.replaceChildren(
      ...options.map((o) => h("option", { value: o.value, selected: o.value === typeFilter }, o.label)),
    );
  };
  filter.addEventListener("change", () => {
    typeFilter = filter.value;
    redraw();
  });
  switcher.addEventListener("change", () => {
    groupBy = switcher.value === "account" ? "account" : switcher.value === "type" ? "type" : "category";
    redraw();
  });
  function redraw(): void {
    if (lastView) {
      table.render(lastView);
      // The rebuilt cells are empty until the valuation is put into them again.
      if (lastOverview) {
        showOverview(lastView, lastOverview);
      }
    }
  }
  table.el
    .querySelector("h2")
    ?.after(
      h("label", { class: "group-by" }, `${t("Group by")} `, switcher),
      h("label", { class: "group-by" }, `${t("Show type")} `, filter),
      editButton(),
    );

  /** In "By category", orders each category's rows by the product's share across all accounts, then by the account's share. */
  function orderByValue(overview: Overview): void {
    const share = new Map(overview.holdings.map((r) => [recordKey(r.account, r.product, r.account_type), Number(r.share)]));
    for (const block of categoryBlocks) {
      const own = (key: string) => share.get(key) ?? 0;
      const byProduct = new Map<number, number>();
      for (const r of block.records) {
        byProduct.set(r.product, (byProduct.get(r.product) ?? 0) + own(r.key));
      }
      const sorted = [...block.records].sort(
        (a, b) =>
          (byProduct.get(b.product) ?? 0) - (byProduct.get(a.product) ?? 0) ||
          a.product - b.product ||
          own(b.key) - own(a.key),
      );
      let previous = block.header;
      for (const r of sorted) {
        previous.after(r.row);
        previous = r.row;
      }
    }
  }

  function showOverview(_view: PortfolioView, overview: Overview): void {
    lastOverview = overview;
    totalCell.textContent = typeFilter === "" ? yen(overview.total) : "";
    note.textContent = "";
    note.classList.remove("error");
    for (const record of overview.holdings) {
      const cells = holdingCells.get(recordKey(record.account, record.product, record.account_type));
      if (cells) {
        cells.value.textContent = yen(record.value);
        cells.share.textContent = percent(record.share);
      }
    }
    orderByValue(overview);
    for (const type of overview.types) {
      const cells = typeCells.get(typeFilterKey(type.account_type));
      if (cells && typeFilter === "") {
        cells.value.textContent = yen(type.value);
        cells.share.textContent = percent(type.share);
      }
    }
    for (const account of overview.accounts) {
      const cells = accountCells.get(account.account);
      if (cells && typeFilter === "") {
        cells.value.textContent = yen(account.value);
        cells.share.textContent = percent(account.share);
      }
    }
    for (const group of overview.groups) {
      const cells = groupCells.get(group.group === null ? "none" : String(group.group));
      if (cells && typeFilter === "") {
        cells.value.textContent = yen(group.current);
        cells.share.textContent = percent(group.current_share);
      }
    }
    for (const category of overview.categories) {
      const cells = categoryCells.get(category.category);
      if (cells && typeFilter === "") {
        cells.value.textContent = yen(category.current);
        cells.share.textContent = percent(category.current_share);
      }
    }
  }

  return {
    el: table.el,
    render(view) {
      lastView = view;
      // A valuation belongs to the state it was worked out from.
      lastOverview = null;
      refill(view);
      table.render(view);
    },
    showOverview,
    overviewFailed(error) {
      lastOverview = null;
      note.classList.add("error");
      note.textContent = t("Could not load: {reason}", { reason: String(error) });
    },
  };
}

/** The price a holding was worked out from, with its unit when it is not 1 (a fund's price per 10,000 units). */
export function priceText(holding: Holding | undefined): string {
  if (!holding || holding.price === null) {
    return "";
  }
  return holding.price_unit > 1 ? `${holding.price} / ${holding.price_unit}` : holding.price;
}
