import { cleanDecimal, formatAmount, fractionToPercent } from "../decimal.ts";
import { h } from "../dom.ts";
import { selectField, textField } from "../fields.ts";
import { getLanguage, t } from "../i18n.ts";
import { panel, type Context, type Panel } from "../panel.ts";
import type { Category, Holding, Overview, PortfolioView } from "../types.ts";
import { groupEntries, groupName } from "./groups.ts";

// The center panel: holding records grouped by category (or by account), each group opened by
// a subtotal row. The amounts are edited in place (in the product's own currency); the JPY
// values and shares come from Rust. A record is one line: the name field, the account, and
// the category select (the way to move a product to another category).

const percent = (fraction: string): string => `${fractionToPercent(fraction)}%`;
const yen = (amount: string): string => formatAmount(amount, getLanguage());

export function holdingsTablePanel(ctx: Context): Panel {
  // Cells that show Rust's valuation, found again by key after each refresh.
  let holdingCells = new Map<string, { value: HTMLElement; share: HTMLElement }>();
  let accountCells = new Map<number, { value: HTMLElement; share: HTMLElement }>();
  let categoryCells = new Map<number, { value: HTMLElement; share: HTMLElement }>();
  // The group subtotals; the key is the group id, or "none" for the ungrouped categories.
  let groupCells = new Map<string, { value: HTMLElement; share: HTMLElement }>();
  const note = h("p", { class: "note", role: "status" });
  let groupBy: "category" | "account" = "category";
  let lastView: PortfolioView | null = null;
  let lastOverview: Overview | null = null;

  const table = panel(
    "Holdings",
    (view) =>
      [
        groupBy,
        view.portfolio.accounts.map((a) => `${a.id}:${a.name}`).join(","),
        view.portfolio.categories.map((c) => `${c.id}:${c.name}:${c.group ?? ""}`).join(","),
        view.portfolio.groups.map((g) => `${g.id}:${g.name}`).join(","),
        // A product's name is followed by its field (no rebuild); its category and currency move or relabel the row.
        view.portfolio.products.map((p) => `${p.id}:${p.category}:${p.currency}`).join(","),
        view.portfolio.holdings.map((x) => `${x.account}:${x.product}`).join(","),
      ].join("|"),
    (view, bind) => {
      holdingCells = new Map();
      categoryCells = new Map();
      groupCells = new Map();
      accountCells = new Map();
      const { categories, products, accounts, holdings } = view.portfolio;
      if (holdings.length === 0) {
        return [h("p", { class: "note" }, t("No holdings yet."))];
      }

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
      const holdingOf = (v: PortfolioView, account: number, product: number) =>
        v.portfolio.holdings.find((x) => x.account === account && x.product === product);

      const rows: HTMLElement[] = [];

      const recordRow = (record: Holding): void => {
        const product = products.find((p) => p.id === record.product);
        const account = accounts.find((a) => a.id === record.account);
        if (!product || !account) {
          return;
        }
        const cells = { value: h("td", { class: "num" }), share: h("td", { class: "num" }) };
        holdingCells.set(`${record.account}:${record.product}`, cells);
        const quantityCell = h("td", { class: "num" });
        const priceCell = h("td", { class: "num" });
        const showQuantityAndPrice = (): void => {
          const held = holdingOf(current, record.account, record.product);
          quantityCell.textContent = held?.quantity ?? "";
          priceCell.textContent = priceText(held);
        };
        // A record that only carries a weight (a product to start buying) has no value yet; the
        // label follows every refresh, since an edit or an import can change either.
        const weightOnlyLabel = h("span", { class: "muted weight-only" }, ` (${t("No value yet")})`);
        const showWeightOnly = (): void => {
          const held = holdingOf(current, record.account, record.product);
          weightOnlyLabel.hidden = !(held && held.weight !== null && Number(held.value) === 0);
        };
        showQuantityAndPrice();
        showWeightOnly();
        refreshers.push(showQuantityAndPrice, showWeightOnly);
        rows.push(
          h(
            "tr",
            {},
            h(
              "th",
              { scope: "row", class: "product-cell" },
              textField({
                ctx,
                bind,
                label: t("Product name"),
                get: (v) => v.portfolio.products.find((p) => p.id === product.id)?.name ?? "",
                edit: (text) => ({ type: "update_product", id: product.id, name: text }),
              }),
            ),
            ...(groupBy === "category" ? [h("td", { class: "clip" }, account.name)] : []),
            h(
              "td",
              {},
              selectField({
                ctx,
                bind,
                label: t("Category of the product"),
                options: categories.map((c) => ({ value: String(c.id), label: c.name })),
                get: (v) => String(v.portfolio.products.find((p) => p.id === product.id)?.category ?? ""),
                edit: (value) => ({ type: "update_product", id: product.id, category: Number(value) }),
              }),
            ),
            h(
              "td",
              {},
              textField({
                ctx,
                bind,
                label: `${product.name} / ${account.name} (${product.currency})`,
                get: (v) => holdingOf(v, record.account, record.product)?.value ?? "",
                edit: (text) => {
                  if (text === "") {
                    return { type: "remove_holding", account: record.account, product: record.product };
                  }
                  const value = cleanDecimal(text);
                  return value === null
                    ? null
                    : {
                        type: "set_holding",
                        account: record.account,
                        product: record.product,
                        value,
                        quantity: holdingOf(current, record.account, record.product)?.quantity ?? null,
                      };
                },
                attrs: { inputmode: "decimal", class: "number" },
              }),
              h("span", { class: "muted" }, ` ${product.currency}`),
              weightOnlyLabel,
            ),
            quantityCell,
            priceCell,
            cells.value,
            cells.share,
          ),
        );
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
              h("th", { scope: "rowgroup", colspan: 2 }, account.name),
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
                { scope: "rowgroup", colspan: 3 },
                textField({
                  ctx,
                  bind,
                  label: t("Category name"),
                  get: (v) => v.portfolio.categories.find((c) => c.id === category.id)?.name ?? "",
                  edit: (text) => ({ type: "update_category", id: category.id, name: text }),
                }),
              ),
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
                { scope: "rowgroup", colspan: 3 },
                id === null
                  ? groupName(view, null)
                  : textField({
                      ctx,
                      bind,
                      label: t("Group name"),
                      get: (v) => v.portfolio.groups.find((g) => g.id === id)?.name ?? "",
                      edit: (text) => ({ type: "rename_group", id, name: text }),
                    }),
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
                h("th", {}, t("Category")),
                h("th", {}, t("Amount")),
                h("th", {}, t("Quantity")),
                h("th", {}, t("Price")),
                h("th", {}, t("Value (JPY)")),
                h("th", {}, t("Share")),
              ),
            ),
            h("tbody", {}, ...rows),
          ),
        ),
      ];
    },
  );
  table.el.append(note);

  const switcher = h(
    "select",
    { "aria-label": t("Group by") },
    h("option", { value: "category" }, t("By category")),
    h("option", { value: "account" }, t("By account")),
  );
  switcher.addEventListener("change", () => {
    groupBy = switcher.value === "account" ? "account" : "category";
    if (lastView) {
      table.render(lastView);
      // The rebuilt cells are empty until the valuation is put into them again.
      if (lastOverview) {
        showOverview(lastView, lastOverview);
      }
    }
  });
  table.el.querySelector("h2")?.after(h("label", { class: "group-by" }, `${t("Group by")} `, switcher));

  function showOverview(_view: PortfolioView, overview: Overview): void {
    lastOverview = overview;
    note.textContent = "";
    note.classList.remove("error");
    for (const record of overview.holdings) {
      const cells = holdingCells.get(`${record.account}:${record.product}`);
      if (cells) {
        cells.value.textContent = yen(record.value);
        cells.share.textContent = percent(record.share);
      }
    }
    for (const account of overview.accounts) {
      const cells = accountCells.get(account.account);
      if (cells) {
        cells.value.textContent = yen(account.value);
        cells.share.textContent = percent(account.share);
      }
    }
    for (const group of overview.groups) {
      const cells = groupCells.get(group.group === null ? "none" : String(group.group));
      if (cells) {
        cells.value.textContent = yen(group.current);
        cells.share.textContent = percent(group.current_share);
      }
    }
    for (const category of overview.categories) {
      const cells = categoryCells.get(category.category);
      if (cells) {
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
function priceText(holding: Holding | undefined): string {
  if (!holding || holding.price === null) {
    return "";
  }
  return holding.price_unit > 1 ? `${holding.price} / ${holding.price_unit}` : holding.price;
}
