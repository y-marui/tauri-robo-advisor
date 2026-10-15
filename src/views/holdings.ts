import { cleanDecimal } from "../decimal.ts";
import { h } from "../dom.ts";
import { textField } from "../fields.ts";
import { t } from "../i18n.ts";
import { panel, type Context, type Panel } from "../panel.ts";
import type { PortfolioView } from "../types.ts";

export function holdingsPanel(ctx: Context): Panel {
  return panel(
    "Holdings",
    (view) =>
      [
        view.portfolio.accounts.map((a) => `${a.id}:${a.name}`).join(","),
        view.portfolio.categories.map((c) => `${c.id}:${c.name}`).join(","),
        view.portfolio.products.map((p) => `${p.id}:${p.name}:${p.category}:${p.currency}`).join(","),
      ].join("|"),
    (view, bind) => {
      const { accounts, products, categories } = view.portfolio;
      if (accounts.length === 0 || products.length === 0) {
        return [h("p", { class: "note" }, t("Add products and accounts to enter holdings."))];
      }

      // Rows follow the category order, so a category's products sit together.
      const ordered = categories.flatMap((category) =>
        products.filter((p) => p.category === category.id),
      );

      let current: PortfolioView = view;
      bind((v) => {
        current = v;
      });

      const rows = ordered.map((product) => {
        const categoryName = categories.find((c) => c.id === product.category)?.name ?? "";
        const cells = accounts.map((account) => {
          const holding = (v: PortfolioView) =>
            v.portfolio.holdings.find((x) => x.account === account.id && x.product === product.id);
          return h(
            "td",
            {},
            textField({
              ctx,
              bind,
              label: `${product.name} / ${account.name}`,
              get: (v) => holding(v)?.value ?? "",
              edit: (text) => {
                if (text === "") {
                  return { type: "remove_holding", account: account.id, product: product.id };
                }
                const value = cleanDecimal(text);
                return value === null
                  ? null
                  : {
                      type: "set_holding",
                      account: account.id,
                      product: product.id,
                      value,
                      quantity: holding(current)?.quantity ?? null,
                    };
              },
              attrs: { inputmode: "decimal", class: "number" },
            }),
          );
        });
        return h(
          "tr",
          {},
          h(
            "th",
            { scope: "row" },
            h("span", { class: "muted" }, `${categoryName} / `),
            product.name,
            h("span", { class: "muted" }, ` (${product.currency})`),
          ),
          ...cells,
        );
      });

      return [
        h("p", { class: "note" }, t("Amounts are in each product’s currency.")),
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
                h("th", {}, t("Products")),
                ...accounts.map((a) => h("th", {}, a.name)),
              ),
            ),
            h("tbody", {}, ...rows),
          ),
        ),
      ];
    },
  );
}
