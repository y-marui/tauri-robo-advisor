import { cleanDecimal } from "../decimal.ts";
import { h } from "../dom.ts";
import { deleteButton, selectField, textField } from "../fields.ts";
import { t } from "../i18n.ts";
import { panel, type Context, type Panel } from "../panel.ts";

const upper = (text: string): string => text.trim().toUpperCase();

export function productsPanel(ctx: Context): Panel {
  return panel(
    "Products",
    (view) =>
      [
        view.portfolio.categories.map((c) => `${c.id}:${c.name}:${c.targeting}`).join(","),
        view.portfolio.products.map((p) => `${p.id}:${p.category}`).join(","),
      ].join("|"),
    (view, bind) => {
      const categories = view.portfolio.categories;
      if (categories.length === 0) {
        return [h("p", { class: "note" }, t("Add a category first."))];
      }
      const categoryOptions = categories.map((c) => ({ value: String(c.id), label: c.name }));

      const rows = view.portfolio.products.map((product) => {
        const id = product.id;
        const find = (v: typeof view) => v.portfolio.products.find((p) => p.id === id);
        const weighted =
          categories.find((c) => c.id === product.category)?.targeting === "weighted";
        return h(
          "tr",
          {},
          h(
            "td",
            {},
            textField({
              ctx,
              bind,
              label: t("Name"),
              get: (v) => find(v)?.name ?? "",
              edit: (text) => ({ type: "update_product", id, name: text }),
            }),
          ),
          h(
            "td",
            {},
            selectField({
              ctx,
              bind,
              label: t("Category"),
              options: categoryOptions,
              get: (v) => String(find(v)?.category ?? ""),
              edit: (value) => ({ type: "update_product", id, category: Number(value) }),
            }),
          ),
          h(
            "td",
            {},
            textField({
              ctx,
              bind,
              label: t("Currency"),
              get: (v) => find(v)?.currency ?? "",
              normalize: upper,
              invalid: t("Currency must be three letters, for example JPY."),
              edit: (text) =>
                /^[A-Z]{3}$/.test(text) ? { type: "update_product", id, currency: text } : null,
              attrs: { class: "currency", maxlength: "3" },
            }),
          ),
          h(
            "td",
            {},
            weighted
              ? textField({
                  ctx,
                  bind,
                  label: t("Relative weight inside the category"),
                  get: (v) => find(v)?.weight ?? "",
                  edit: (text) => {
                    if (text === "") {
                      return { type: "set_product_weight", id, weight: null };
                    }
                    const clean = cleanDecimal(text);
                    return clean === null
                      ? null
                      : { type: "set_product_weight", id, weight: clean };
                  },
                  attrs: { inputmode: "decimal", class: "number" },
                })
              : null,
          ),
          h("td", {}, deleteButton(() => void ctx.apply({ type: "remove_product", id }))),
        );
      });

      const name = h("input", { type: "text", "aria-label": t("Name"), autocomplete: "off" });
      const category = h(
        "select",
        { "aria-label": t("Category") },
        ...categoryOptions.map((o) => h("option", { value: o.value }, o.label)),
      );
      const currency = h("input", {
        type: "text",
        "aria-label": t("Currency"),
        class: "currency",
        maxlength: "3",
        value: "JPY",
      });
      const form = h(
        "form",
        { class: "add-row" },
        name,
        category,
        currency,
        h("button", { type: "submit" }, t("Add a product")),
      );
      form.addEventListener("submit", (event) => {
        event.preventDefault();
        void ctx
          .apply({
            type: "add_product",
            name: name.value,
            category: Number(category.value),
            currency: upper(currency.value),
          })
          .then((ok) => {
            if (ok) {
              name.value = "";
              name.focus();
            }
          });
      });

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
                h("th", {}, t("Name")),
                h("th", {}, t("Category")),
                h("th", {}, t("Currency")),
                h("th", {}, t("Weight")),
                h("th", {}),
              ),
            ),
            h("tbody", {}, ...rows),
          ),
        ),
        form,
      ];
    },
  );
}
