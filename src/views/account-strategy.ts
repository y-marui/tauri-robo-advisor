import { cleanDecimal } from "../decimal.ts";
import { h } from "../dom.ts";
import { t } from "../i18n.ts";
import type { Bind, Context } from "../panel.ts";
import type { PortfolioView, Product, Strategy } from "../types.ts";

// The strategy of one account in the settings: follow the portfolio's targets, or aim at a list
// of products with ratios typed by hand (a blank ratio means an equal share). Validation lives in
// Rust; the editor only builds the edit and shows the error that comes back.

export function strategyEditor(
  ctx: Context,
  bind: (binding: Bind) => void,
  account: number,
  products: Product[],
): HTMLElement {
  const mode = h(
    "select",
    { "aria-label": t("Strategy") },
    h("option", { value: "portfolio" }, t("Follow the portfolio")),
    h("option", { value: "products" }, t("Specified products with ratios")),
  );
  const rows = products.map((product) => ({
    id: product.id,
    box: h("input", { type: "checkbox", "aria-label": product.name }),
    ratio: h("input", {
      type: "text",
      inputmode: "decimal",
      class: "number",
      autocomplete: "off",
      "aria-label": t("Ratio of {product}", { product: product.name }),
      placeholder: t("Equal"),
    }),
  }));
  // The list is shown as soon as "specified products" is chosen, before any product is ticked
  // (a strategy needs a product, so nothing is saved until one is).
  let wantProducts = false;

  const list = h(
    "div",
    {},
    h("p", { class: "note" }, t("Tick the products and type a ratio for each (blank = equal share). Category targets are not used for this account, and products outside the list are neither bought nor sold.")),
    ...rows.map((row, index) =>
      h("label", { class: "check" }, row.box, products[index]?.name ?? "", " ", row.ratio),
    ),
  );

  const send = (): void => {
    const ticked = rows.filter((row) => row.box.checked);
    if (mode.value === "portfolio") {
      void ctx.apply({ type: "set_strategy", id: account, strategy: { type: "portfolio" } });
      return;
    }
    if (ticked.length === 0) {
      // Unticking the last product leaves the strategy as it was saved until another is ticked.
      return;
    }
    const items: { product: number; ratio: string | null }[] = [];
    for (const row of ticked) {
      const text = row.ratio.value.trim();
      const ratio = text === "" ? null : cleanDecimal(text);
      if (text !== "" && ratio === null) {
        ctx.fail(t("Enter a valid number."));
        return;
      }
      items.push({ product: row.id, ratio });
    }
    void ctx.apply({ type: "set_strategy", id: account, strategy: { type: "products", items } });
  };

  mode.addEventListener("change", () => {
    wantProducts = mode.value === "products";
    list.hidden = !wantProducts;
    send();
  });
  for (const row of rows) {
    row.box.addEventListener("change", send);
    row.ratio.addEventListener("change", send);
  }

  bind((view: PortfolioView) => {
    const strategy: Strategy = view.portfolio.accounts.find((a) => a.id === account)?.strategy ?? {
      type: "portfolio",
    };
    if (strategy.type === "products") {
      wantProducts = true;
    }
    mode.value = wantProducts ? "products" : "portfolio";
    list.hidden = !wantProducts;
    for (const row of rows) {
      const item = strategy.type === "products" ? strategy.items.find((i) => i.product === row.id) : undefined;
      row.box.checked = item !== undefined;
      if (document.activeElement !== row.ratio) {
        row.ratio.value = item?.ratio ?? "";
      }
    }
  });

  return h("details", {}, h("summary", {}, t("Strategy")), mode, list);
}
