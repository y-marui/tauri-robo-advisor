import { h } from "../dom.ts";
import { amountField, checkboxField, deleteButton, orderButtons, textField } from "../fields.ts";
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

        // "Any product" means no restriction; otherwise the ticked products are
        // the only ones this account can buy.
        const anyBox = h("input", { type: "checkbox", "aria-label": t("Any product") });
        const productBoxes = products.map((product) => ({
          id: product.id,
          box: h("input", { type: "checkbox", "aria-label": product.name }),
        }));
        const send = () => {
          const ids = productBoxes.filter((p) => p.box.checked).map((p) => p.id);
          void ctx.apply({ type: "set_tradable", id, products: anyBox.checked ? null : ids });
        };
        anyBox.addEventListener("change", () => {
          if (!anyBox.checked) {
            // Start from "everything ticked" so unticking "Any" changes nothing yet.
            for (const p of productBoxes) {
              p.box.checked = true;
            }
          }
          send();
        });
        for (const p of productBoxes) {
          p.box.addEventListener("change", send);
        }
        bind((v) => {
          const tradable = find(v)?.tradable ?? null;
          anyBox.checked = tradable === null;
          for (const p of productBoxes) {
            p.box.checked = tradable === null || tradable.includes(p.id);
            p.box.disabled = tradable === null;
          }
        });
        const canBuy = h(
          "details",
          {},
          h("summary", {}, t("Can buy")),
          h("label", { class: "check" }, anyBox, t("Any product")),
          ...productBoxes.map((p, index) =>
            h("label", { class: "check" }, p.box, products[index]?.name ?? ""),
          ),
        );

        // Products ticked here are never bought or sold by a suggestion in this account (their
        // holdings still count toward the category).
        const excludedBoxes = products.map((product) => ({
          id: product.id,
          box: h("input", { type: "checkbox", "aria-label": product.name }),
        }));
        for (const p of excludedBoxes) {
          p.box.addEventListener("change", () => {
            const ids = excludedBoxes.filter((x) => x.box.checked).map((x) => x.id);
            void ctx.apply({ type: "set_excluded", id, products: ids });
          });
        }
        bind((v) => {
          const excluded = find(v)?.excluded ?? [];
          for (const p of excludedBoxes) {
            p.box.checked = excluded.includes(p.id);
          }
        });
        const leftOut = h(
          "details",
          {},
          h("summary", {}, t("Left out of trading")),
          ...excludedBoxes.map((p, index) =>
            h("label", { class: "check" }, p.box, products[index]?.name ?? ""),
          ),
        );

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
          h(
            "td",
            { class: "center" },
            checkboxField({
              ctx,
              bind,
              label: t("Sellable"),
              get: (v) => find(v)?.sellable ?? false,
              edit: (checked) => ({ type: "update_account", id, sellable: checked }),
            }),
          ),
          h(
            "td",
            {},
            amountField({
              ctx,
              bind,
              label: t("Rounding unit"),
              get: (v) => find(v)?.rounding_unit ?? "",
              save: (value) => ({ type: "update_account", id, rounding_unit: value }),
            }),
          ),
          h("td", {}, canBuy),
          h("td", {}, leftOut),
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
          h("td", {}),
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
                h("th", {}, t("Sellable")),
                h("th", {}, t("Rounding unit")),
                h("th", {}, t("Can buy")),
                h("th", {}, t("Left out of trading")),
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
