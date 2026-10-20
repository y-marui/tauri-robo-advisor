import { percentToFraction } from "../decimal.ts";
import { h } from "../dom.ts";
import { deleteButton, orderButtons, percentField, selectField, textField } from "../fields.ts";
import { t } from "../i18n.ts";
import { grip, makeSortable } from "../sortable.ts";
import { panel, type Context, type Panel } from "../panel.ts";
import type { ProductTargeting } from "../types.ts";
import { TARGETINGS, targetingLabel } from "./labels.ts";

export function categoriesPanel(ctx: Context): Panel {
  return panel(
    "Categories",
    (view) =>
      view.portfolio.categories.map((c) => c.id).join(",") +
      "|" +
      view.portfolio.groups.map((g) => `${g.id}:${g.name}`).join(","),
    (view, bind) => {
      const ids = view.portfolio.categories.map((c) => c.id);
      const rows = view.portfolio.categories.map((category, position) => {
        const id = category.id;
        const find = (v: typeof view) => v.portfolio.categories.find((c) => c.id === id);
        return h(
          "tr",
          {},
          h("td", { class: "grip-cell" }, grip()),
          h(
            "td",
            {},
            textField({
              ctx,
              bind,
              label: t("Name"),
              get: (v) => find(v)?.name ?? "",
              edit: (text) => ({ type: "update_category", id, name: text }),
            }),
          ),
          ...(view.portfolio.groups.length === 0
            ? []
            : [
                h(
                  "td",
                  {},
                  selectField({
                    ctx,
                    bind,
                    label: t("Group of the category"),
                    options: [
                      { value: "", label: t("Ungrouped") },
                      ...view.portfolio.groups.map((g) => ({ value: String(g.id), label: g.name })),
                    ],
                    get: (v) => String(find(v)?.group ?? ""),
                    edit: (value) => ({
                      type: "set_category_group",
                      category: id,
                      group: value === "" ? null : Number(value),
                    }),
                  }),
                ),
              ]),
          h(
            "td",
            {},
            percentField({
              ctx,
              bind,
              label: t("Target (%)"),
              get: (v) => find(v)?.target_weight ?? "0",
              save: (fraction) => ({ type: "update_category", id, target_weight: fraction }),
            }),
          ),
          h(
            "td",
            {},
            selectField({
              ctx,
              bind,
              label: t("Product targeting"),
              options: TARGETINGS.map((value) => ({ value, label: targetingLabel(value) })),
              get: (v) => find(v)?.targeting ?? "free",
              edit: (value) => ({
                type: "update_category",
                id,
                targeting: value as ProductTargeting,
              }),
            }),
          ),
          h(
            "td",
            {},
            orderButtons(ctx, position, view.portfolio.categories.length, (up) => ({
              type: "move_category",
              id,
              up,
            }), { list: "categories", id }),
            deleteButton(() => void ctx.apply({ type: "remove_category", id })),
          ),
        );
      });

      const tbody = h("tbody", {}, ...rows);
      makeSortable({
        list: "categories",
        tbody,
        ids,
        move: (id, index) => void ctx.apply({ type: "move_category_to", id, index }),
      });

      const name = h("input", { type: "text", "aria-label": t("Name"), autocomplete: "off" });
      const target = h("input", {
        type: "text",
        "aria-label": t("Target (%)"),
        inputmode: "decimal",
        class: "number",
        placeholder: "%",
      });
      const form = h(
        "form",
        { class: "add-row" },
        name,
        target,
        h("button", { type: "submit" }, t("Add a category")),
      );
      form.addEventListener("submit", (event) => {
        event.preventDefault();
        const fraction = percentToFraction(target.value || "0");
        if (fraction === null) {
          ctx.fail(t("Enter a valid number."));
          return;
        }
        void ctx
          .apply({ type: "add_category", name: name.value, target_weight: fraction })
          .then((ok) => {
            if (ok) {
              name.value = "";
              target.value = "";
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
                h("th", {}),
                h("th", {}, t("Name")),
                ...(view.portfolio.groups.length === 0 ? [] : [h("th", {}, t("Group"))]),
                h("th", {}, t("Target (%)")),
                h("th", {}, t("Product targeting")),
                h("th", {}),
              ),
            ),
            tbody,
          ),
        ),
        form,
      ];
    },
  );
}
