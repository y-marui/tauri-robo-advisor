import { h } from "../dom.ts";
import { t } from "../i18n.ts";
import type { Bind, Context } from "../panel.ts";
import { type DraftProblem, strategyFromDraft } from "../strategy-draft.ts";
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
    h("p", { class: "note" }, t("Tick the products and type a ratio for each (blank = equal share), then save. Category targets are not used for this account, and products outside the list are neither bought nor sold.")),
    ...rows.map((row, index) =>
      h("label", { class: "check" }, row.box, products[index]?.name ?? "", " ", row.ratio),
    ),
  );

  // Edits are a draft until "Save" (enabled only when the draft is consistent, with the reason shown
  // otherwise), so the saved strategy never passes through an invalid state.
  let dirty = false;
  const save = h("button", { type: "button" }, t("Save strategy"));
  const reason = h("p", { class: "note" });

  const problemText = (problem: DraftProblem): string => {
    switch (problem) {
      case "no_product":
        return t("Tick at least one product.");
      case "mixed_ratios":
        return t("Give a ratio to every ticked product, or leave them all blank.");
      case "invalid_number":
        return t("Enter a valid number.");
      case "not_positive":
        return t("Ratios must be positive.");
    }
  };

  const draft = () =>
    strategyFromDraft(rows.map((row) => ({ id: row.id, checked: row.box.checked, ratio: row.ratio.value })));

  const refreshSave = (): void => {
    const result = draft();
    const editing = mode.value === "products" && dirty;
    save.hidden = !editing;
    save.disabled = !result.ok;
    reason.textContent = editing && !result.ok ? problemText(result.problem) : "";
  };

  const touch = (): void => {
    dirty = true;
    refreshSave();
  };

  save.addEventListener("click", () => {
    const result = draft();
    if (!result.ok) {
      return;
    }
    save.disabled = true;
    void ctx
      .apply({ type: "set_strategy", id: account, strategy: { type: "products", items: result.items } })
      .then((ok) => {
        if (ok) {
          dirty = false;
        }
      })
      .finally(refreshSave);
  });

  mode.addEventListener("change", () => {
    wantProducts = mode.value === "products";
    list.hidden = !wantProducts;
    if (wantProducts) {
      // The list is edited as a draft; the saved strategy is unchanged until it is saved.
      dirty = true;
      refreshSave();
    } else {
      dirty = false;
      refreshSave();
      void ctx.apply({ type: "set_strategy", id: account, strategy: { type: "portfolio" } });
    }
  });
  for (const row of rows) {
    row.box.addEventListener("change", touch);
    row.ratio.addEventListener("input", touch);
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
    // A draft in progress is not overwritten by a refresh.
    if (!dirty) {
      for (const row of rows) {
        const item = strategy.type === "products" ? strategy.items.find((i) => i.product === row.id) : undefined;
        row.box.checked = item !== undefined;
        row.ratio.value = item?.ratio ?? "";
      }
    }
    refreshSave();
  });

  return h("details", {}, h("summary", {}, t("Strategy")), mode, list, reason, save);
}
