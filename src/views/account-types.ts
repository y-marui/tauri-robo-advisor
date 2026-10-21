import { h } from "../dom.ts";
import { checkboxField, deleteButton, textField } from "../fields.ts";
import { t } from "../i18n.ts";
import { BUILTIN_TYPES, typeLabel } from "./labels.ts";
import { panel, type Context, type Panel } from "../panel.ts";

/**
 * The account types: the built-in ones (their names are fixed; only whether a suggestion may sell
 * them can be changed) and the ones the user added. A type belongs to a record, set in the
 * Records card.
 */
export function accountTypesPanel(ctx: Context): Panel {
  return panel(
    "Account types",
    (view) => (view.portfolio.custom_types ?? []).map((c) => c.id).join(","),
    (view, bind) => {
      const builtinRows = BUILTIN_TYPES.map((id) =>
        h(
          "tr",
          {},
          h("td", {}, typeLabel(id, undefined)),
          h(
            "td",
            { class: "center" },
            checkboxField({
              ctx,
              bind,
              label: `${typeLabel(id, undefined)}: ${t("Sellable")}`,
              get: (v) => v.builtin_type_sellable[id] ?? true,
              edit: (checked) => ({ type: "set_type_sellable", id, sellable: checked }),
            }),
          ),
        ),
      );
      const rows = (view.portfolio.custom_types ?? []).map((type) => {
        const id = type.id;
        const find = (v: typeof view) => (v.portfolio.custom_types ?? []).find((c) => c.id === id);
        return h(
          "tr",
          {},
          h(
            "td",
            {},
            textField({
              ctx,
              bind,
              label: t("Type name"),
              get: (v) => find(v)?.name ?? "",
              edit: (text) => ({ type: "update_account_type", id, name: text }),
            }),
          ),
          h(
            "td",
            { class: "center" },
            checkboxField({
              ctx,
              bind,
              label: t("Sellable"),
              get: (v) => find(v)?.sellable ?? false,
              edit: (checked) => ({ type: "set_type_sellable", id, sellable: checked }),
            }),
          ),
          h("td", {}, deleteButton(() => void ctx.apply({ type: "remove_account_type", id }), t("Delete type"))),
        );
      });

      const name = h("input", { type: "text", "aria-label": t("Type name"), autocomplete: "off" });
      const sellable = h("input", { type: "checkbox", "aria-label": t("Sellable"), checked: true });
      const form = h(
        "form",
        { class: "add-row" },
        name,
        h("label", { class: "check" }, sellable, t("Sellable")),
        h("button", { type: "submit" }, t("Add a type")),
      );
      form.addEventListener("submit", (event) => {
        event.preventDefault();
        void ctx.apply({ type: "add_account_type", name: name.value, sellable: sellable.checked }).then((ok) => {
          if (ok) {
            name.value = "";
            name.focus();
          }
        });
      });

      return [
        h(
          "p",
          { class: "muted" },
          t(
            "Each record has a type (general, specific, NISA, NISA growth, NISA tsumitate, old NISA, cash, crypto assets, or one added here). “NISA” means a NISA whose frame is not decided yet. The type is read automatically only where the page shows it; otherwise set it by hand in the Records card.",
          ),
          " ",
          t(
            "The sellable box says whether a suggestion may sell records of the type. NISA types are off by default, because selling gives up tax-free room. When a product is held in several types of one account, a sale takes from the other types first and from the NISA types last.",
          ),
        ),
        h("h3", {}, t("Built-in types")),
        h(
          "div",
          { class: "table-wrap" },
          h(
            "table",
            {},
            h("thead", {}, h("tr", {}, h("th", {}, t("Name")), h("th", {}, t("Sellable")))),
            h("tbody", {}, ...builtinRows),
          ),
        ),
        h("h3", {}, t("Types you added")),
        ...(rows.length > 0
          ? [
              h(
                "div",
                { class: "table-wrap" },
                h(
                  "table",
                  {},
                  h("thead", {}, h("tr", {}, h("th", {}, t("Name")), h("th", {}, t("Sellable")), h("th", {}))),
                  h("tbody", {}, ...rows),
                ),
              ),
            ]
          : []),
        form,
      ];
    },
  );
}
