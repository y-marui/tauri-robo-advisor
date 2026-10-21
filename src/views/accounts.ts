import { h } from "../dom.ts";
import { deleteButton, orderButtons, textField } from "../fields.ts";
import { t } from "../i18n.ts";
import { strategyEditor } from "./account-strategy.ts";
import { grip, makeSortable } from "../sortable.ts";
import { panel, type Context, type Panel } from "../panel.ts";

export function accountsPanel(ctx: Context): Panel {
  return panel(
    "Accounts",
    (view) =>
      [
        view.portfolio.accounts.map((a) => a.id).join(","),
        view.portfolio.products.map((p) => `${p.id}:${p.name}`).join(","),
      ].join("|"),
    (view, bind) => {
      const products = view.portfolio.products;
      const rows = view.portfolio.accounts.map((account, position) => {
        const id = account.id;
        const find = (v: typeof view) => v.portfolio.accounts.find((a) => a.id === id);

        // The order of the accounts is the order everywhere (the tabs, the tables, the choices).
        const order = orderButtons(ctx, position, view.portfolio.accounts.length, (up) => ({
          type: "move_account",
          id,
          up,
        }), { list: "accounts", id });

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
              edit: (text) => ({ type: "update_account", id, name: text }),
            }),
          ),
          h("td", {}, strategyEditor(ctx, bind, id, products)),
          h("td", {}, order, deleteButton(() => void ctx.apply({ type: "remove_account", id }))),
        );
      });

      const tbody = h("tbody", {}, ...rows);
      makeSortable({
        list: "accounts",
        tbody,
        ids: view.portfolio.accounts.map((x) => x.id),
        move: (id, index) => void ctx.apply({ type: "move_account_to", id, index }),
      });

      // The add form is the last row of the table; its controls belong to this (empty) form.
      const formId = "add-account-form";
      const name = h("input", { type: "text", "aria-label": t("Name"), autocomplete: "off", form: formId });
      const form = h("form", { id: formId });
      const addRow = h(
        "tfoot",
        {},
        h(
          "tr",
          {},
          h("td", {}),
          h("td", {}, name),
          h("td", {}),
          h("td", {}),
          h("td", { class: "add-cell" }, h("button", { type: "submit", form: formId }, t("Add an account"))),
        ),
      );
      form.addEventListener("submit", (event) => {
        event.preventDefault();
        void ctx
          .apply({ type: "add_account", name: name.value })
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
            { class: "list-table" },
            h(
              "thead",
              {},
              h(
                "tr",
                {},
                h("th", {}),
                h("th", {}, t("Name")),
                h("th", {}, t("Strategy")),
                h("th", {}),
              ),
            ),
            tbody,
            addRow,
          ),
        ),
        form,
      ];
    },
  );
}
