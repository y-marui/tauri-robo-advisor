import { h } from "../dom.ts";
import { checkboxField, deleteButton, textField } from "../fields.ts";
import { t } from "../i18n.ts";
import { panel, type Context, type Panel } from "../panel.ts";

/**
 * The account types the user added. The built-in types (general, specific, NISA, old NISA,
 * deposit, crypto assets) are fixed; a type belongs to a record, set in the Records card.
 */
export function accountTypesPanel(ctx: Context): Panel {
  return panel(
    "Account types",
    (view) => (view.portfolio.custom_types ?? []).map((c) => c.id).join(","),
    (view, bind) => {
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
              edit: (checked) => ({ type: "update_account_type", id, sellable: checked }),
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
            "Each record has a type (general, specific, NISA, NISA growth, NISA tsumitate, old NISA, cash, crypto assets, or one added here). NISA types, old NISA included, are never sold by a suggestion. “NISA” means a NISA whose frame is not decided yet. The type is read automatically only where the page shows it; otherwise set it by hand in the Records card.",
          ),
          " ",
          t("Types you add: the sellable box says whether a suggestion may sell records of the type."),
        ),
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
