import { cleanDecimal, formatAmount, fractionToPercent } from "../decimal.ts";
import { h } from "../dom.ts";
import { selectField, textField } from "../fields.ts";
import { getLanguage, t } from "../i18n.ts";
import { panel, type Context, type Panel } from "../panel.ts";
import type { Holding, Overview, PortfolioView } from "../types.ts";

// The center panel: holding records grouped by category, each group closed by
// a subtotal row. The amounts are edited in place (in the product's own
// currency); the JPY values, shares, and differences come from Rust.

const percent = (fraction: string): string => `${fractionToPercent(fraction)}%`;
const yen = (amount: string): string => formatAmount(amount, getLanguage());

/** "+17.78" or "-5" percentage points; the sign carries the meaning, not only the color. */
function points(difference: string): string {
  const text = fractionToPercent(difference);
  return text.startsWith("-") || text === "0" ? text : `+${text}`;
}

/** Below target would be corrected by buying, above target by selling. */
function differenceClass(difference: string): string {
  return difference.startsWith("-") ? "delta-buy" : difference === "0" ? "" : "delta-sell";
}

export function holdingsTablePanel(ctx: Context): Panel {
  // Cells that show Rust's valuation, found again by key after each refresh.
  let holdingCells = new Map<string, { value: HTMLElement; share: HTMLElement }>();
  let categoryCells = new Map<
    number,
    { value: HTMLElement; share: HTMLElement; target: HTMLElement; difference: HTMLElement }
  >();
  const note = h("p", { class: "note", role: "status" });

  const table = panel(
    "Holdings by category",
    (view) =>
      [
        view.portfolio.accounts.map((a) => `${a.id}:${a.name}`).join(","),
        view.portfolio.categories.map((c) => `${c.id}:${c.name}`).join(","),
        view.portfolio.products.map((p) => `${p.id}:${p.name}:${p.category}:${p.currency}`).join(","),
        view.portfolio.holdings.map((x) => `${x.account}:${x.product}`).join(","),
      ].join("|"),
    (view, bind) => {
      holdingCells = new Map();
      categoryCells = new Map();
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
      for (const category of categories) {
        const own = products.filter((p) => p.category === category.id);
        const records = holdings.filter((x) => own.some((p) => p.id === x.product));

        const subtotal = {
          value: h("td", { class: "num" }),
          share: h("td", { class: "num" }),
          target: h("td", { class: "num" }),
          difference: h("td", { class: "num" }),
        };
        categoryCells.set(category.id, subtotal);
        rows.push(
          h(
            "tr",
            { class: "group" },
            h(
              "th",
              { scope: "rowgroup", colspan: 2 },
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
            subtotal.target,
            subtotal.difference,
          ),
        );

        for (const record of records) {
          const product = own.find((p) => p.id === record.product);
          const account = accounts.find((a) => a.id === record.account);
          if (!product || !account) {
            continue;
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
          showQuantityAndPrice();
          refreshers.push(showQuantityAndPrice);
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
                selectField({
                  ctx,
                  bind,
                  label: t("Category of the product"),
                  options: categories.map((c) => ({ value: String(c.id), label: c.name })),
                  get: (v) => String(v.portfolio.products.find((p) => p.id === product.id)?.category ?? ""),
                  edit: (value) => ({ type: "update_product", id: product.id, category: Number(value) }),
                }),
              ),
              h("td", {}, account.name),
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
              ),
              quantityCell,
              priceCell,
              cells.value,
              cells.share,
              h("td", {}),
              h("td", {}),
            ),
          );
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
                h("th", {}, t("Account")),
                h("th", {}, t("Amount")),
                h("th", {}, t("Quantity")),
                h("th", {}, t("Price")),
                h("th", {}, t("Value (JPY)")),
                h("th", {}, t("Share")),
                h("th", {}, t("Target")),
                h("th", {}, t("Difference (points)")),
              ),
            ),
            h("tbody", {}, ...rows),
          ),
        ),
      ];
    },
  );
  table.el.append(note);

  return {
    el: table.el,
    render: table.render,
    showOverview(_view: PortfolioView, overview: Overview) {
      note.textContent = "";
      note.classList.remove("error");
      for (const record of overview.holdings) {
        const cells = holdingCells.get(`${record.account}:${record.product}`);
        if (cells) {
          cells.value.textContent = yen(record.value);
          cells.share.textContent = percent(record.share);
        }
      }
      for (const category of overview.categories) {
        const cells = categoryCells.get(category.category);
        if (cells) {
          cells.value.textContent = yen(category.current);
          cells.share.textContent = percent(category.current_share);
          cells.target.textContent = percent(category.target_share);
          cells.difference.textContent = points(category.difference);
          cells.difference.className = `num ${differenceClass(category.difference)}`;
        }
      }
    },
    overviewFailed(error) {
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
