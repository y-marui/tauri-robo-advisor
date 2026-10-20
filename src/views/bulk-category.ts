import { h } from "../dom.ts";
import { t } from "../i18n.ts";
import type { Context } from "../panel.ts";
import type { Category, Edit, Product } from "../types.ts";

/** The value of the "new category" choice; real categories are numbers. */
export const NEW_CATEGORY = "new";

/** The ids of the products that are in `category` and shown (a hidden one is not selected). */
export function productsOfCategory(products: Product[], category: number, shown: Set<number>): number[] {
  return products.filter((p) => p.category === category && shown.has(p.id)).map((p) => p.id);
}

/**
 * The edit that gives the selected products one category: an existing one, or (choice
 * `NEW_CATEGORY`) a new one with the given name. Null while nothing is selected or the name is empty.
 */
export function bulkCategoryEdit(selected: number[], choice: string, newName: string): Edit | null {
  if (selected.length === 0) {
    return null;
  }
  if (choice === NEW_CATEGORY) {
    const name = newName.trim();
    return name === "" ? null : { type: "set_products_category", products: selected, new_category: name };
  }
  const category = Number(choice);
  return Number.isInteger(category) ? { type: "set_products_category", products: selected, category } : null;
}

/** The toolbar above the Records table: select products, then give them one category. */
export function bulkCategoryBar(
  ctx: Context,
  categories: Category[],
  products: Product[],
  selected: Set<number>,
  shown: () => Set<number>,
  sync: () => void,
): { el: HTMLElement; refresh: () => void } {
  const count = h("span", { class: "muted", role: "status" });
  const button = (label: string, onClick: () => void): HTMLButtonElement => {
    const b = h("button", { type: "button" }, label);
    b.addEventListener("click", onClick);
    return b;
  };
  const pick = h(
    "select",
    { "aria-label": t("Select the products of") },
    ...categories.map((c) => h("option", { value: String(c.id) }, c.name)),
  );
  const target = h(
    "select",
    { "aria-label": t("Category of the selected products") },
    ...categories.map((c) => h("option", { value: String(c.id) }, c.name)),
    h("option", { value: NEW_CATEGORY }, t("New category...")),
  );
  const newName = h("input", {
    type: "text",
    "aria-label": t("New category name"),
    placeholder: t("New category name"),
    autocomplete: "off",
  });
  const apply = h("button", { type: "submit" }, t("Apply to the selection"));
  const refresh = (): void => {
    count.textContent = t("{count} selected", { count: String(selected.size) });
    newName.hidden = target.value !== NEW_CATEGORY;
    apply.disabled = bulkCategoryEdit([...selected], target.value, newName.value) === null;
  };
  target.addEventListener("change", refresh);
  newName.addEventListener("input", refresh);

  const form = h("form", { class: "add-row" }, target, newName, apply);
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    const edit = bulkCategoryEdit([...selected], target.value, newName.value);
    if (edit === null) {
      return;
    }
    void ctx.apply(edit).then((ok) => {
      if (ok) {
        selected.clear();
        newName.value = "";
        sync();
      }
    });
  });

  const el = h(
    "div",
    { class: "bulk-bar" },
    h(
      "div",
      { class: "add-row" },
      button(t("Select all shown"), () => {
        for (const id of shown()) {
          selected.add(id);
        }
        sync();
      }),
      button(t("Select none"), () => {
        selected.clear();
        sync();
      }),
      pick,
      button(t("Select"), () => {
        for (const id of productsOfCategory(products, Number(pick.value), shown())) {
          selected.add(id);
        }
        sync();
      }),
      count,
    ),
    form,
  );
  refresh();
  return { el, refresh };
}
