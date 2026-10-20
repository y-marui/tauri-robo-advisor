import { h } from "../dom.ts";
import { t } from "../i18n.ts";
import type { Context } from "../panel.ts";
import type { Edit, PortfolioView, Product } from "../types.ts";

/** The products another product can be merged into: the same currency (Rust refuses the rest). */
export function mergeTargets(products: Product[], product: Product): Product[] {
  return products.filter((p) => p.id !== product.id && p.currency === product.currency);
}

/** The accounts that hold a record of both products (the merge needs to know which one stays). */
export function mergeClashes(view: PortfolioView, a: number, b: number): number[] {
  const holders = (product: number): number[] =>
    view.portfolio.holdings.filter((r) => r.product === product).map((r) => r.account);
  const other = new Set(holders(b));
  return holders(a).filter((account) => other.has(account));
}

/** The edit that cuts an alias out as a new product; null when the name or the alias is missing. */
export function splitEdit(
  product: number,
  alias: { source: string; name: string } | undefined,
  name: string,
  accounts: number[],
): Edit | null {
  const trimmed = name.trim();
  if (alias === undefined || trimmed === "") {
    return null;
  }
  return { type: "split_product", product, source: alias.source, alias: alias.name, name: trimmed, accounts };
}

/**
 * The "Merge or split" fold of one product: merge it into another product of the same currency,
 * or cut one of its aliases out as a new product together with the records picked.
 */
export function productTools(ctx: Context, view: PortfolioView, product: Product): HTMLElement {
  const { products, accounts, holdings } = view.portfolio;
  const parts: HTMLElement[] = [];

  const targets = mergeTargets(products, product);
  if (targets.length > 0) {
    const target = h(
      "select",
      { "aria-label": `${product.name}: ${t("Merge into")}` },
      ...targets.map((p) => h("option", { value: String(p.id), title: p.name }, p.name)),
    );
    const policy = h(
      "select",
      { "aria-label": `${product.name}: ${t("Record in both")}` },
      h("option", { value: "keep" }, t("Keep the record of the other product")),
      h("option", { value: "take" }, t("Take the record of this product")),
    );
    const form = h(
      "form",
      { class: "record-add" },
      target,
      policy,
      h("button", { type: "submit" }, t("Merge")),
    );
    form.addEventListener("submit", (event) => {
      event.preventDefault();
      void ctx.apply({
        type: "merge_products",
        keep: Number(target.value),
        remove: product.id,
        on_conflict: policy.value === "take" ? "take" : "keep",
      });
    });
    parts.push(
      h("p", { class: "note" }, t("Merging moves the aliases and the records to the other product and deletes this one.")),
      form,
    );
  }

  const holding = accounts.filter((a) => holdings.some((r) => r.product === product.id && r.account === a.id));
  if (product.aliases.length > 0) {
    const alias = h(
      "select",
      { "aria-label": `${product.name}: ${t("Alias to split off")}` },
      ...product.aliases.map((a, i) => h("option", { value: String(i) }, `${a.source}: ${a.name}`)),
    );
    const name = h("input", { type: "text", "aria-label": t("Name of the new product"), placeholder: t("Name of the new product"), autocomplete: "off" });
    const picks = holding.map((a) => ({ account: a.id, box: h("input", { type: "checkbox" }) }));
    const form = h(
      "form",
      { class: "record-add" },
      alias,
      name,
      ...picks.map((p) =>
        h("label", {}, p.box, ` ${accounts.find((a) => a.id === p.account)?.name ?? ""}`),
      ),
      h("button", { type: "submit" }, t("Split off")),
    );
    form.addEventListener("submit", (event) => {
      event.preventDefault();
      const edit = splitEdit(
        product.id,
        product.aliases[Number(alias.value)],
        name.value,
        picks.filter((p) => p.box.checked).map((p) => p.account),
      );
      if (edit === null) {
        ctx.fail(t("Enter a name for the new product."));
        return;
      }
      void ctx.apply(edit);
    });
    parts.push(
      h("p", { class: "note" }, t("Splitting cuts an alias out as a new product. Tick the accounts whose records move with it.")),
      form,
    );
  }

  if (parts.length === 0) {
    return h("span");
  }
  return h("details", { class: "product-tools" }, h("summary", {}, t("Merge or split")), ...parts);
}
