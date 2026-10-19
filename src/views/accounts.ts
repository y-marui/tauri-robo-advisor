import { h } from "../dom.ts";
import { amountField, checkboxField, deleteButton, selectField, textField } from "../fields.ts";
import { t } from "../i18n.ts";
import { panel, type Context, type Panel } from "../panel.ts";
import type { AccountKind } from "../types.ts";
import { KINDS, kindLabel } from "./labels.ts";

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

        // The order of the accounts is the order everywhere (the tabs, the tables, the choices).
        const up = h("button", { type: "button", "aria-label": t("Move up") }, "↑");
        const down = h("button", { type: "button", "aria-label": t("Move down") }, "↓");
        up.addEventListener("click", () => void ctx.apply({ type: "move_account", id, up: true }));
        down.addEventListener("click", () => void ctx.apply({ type: "move_account", id, up: false }));
        // At an end there is nothing to move: the button is off (a move there would only save for nothing).
        up.disabled = position === 0;
        down.disabled = position === view.portfolio.accounts.length - 1;
        const order = h("span", { class: "order" }, up, down);

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
              edit: (text) => ({ type: "update_account", id, name: text }),
            }),
          ),
          h(
            "td",
            {},
            selectField({
              ctx,
              bind,
              label: t("Type"),
              options: KINDS.map((value) => ({ value, label: kindLabel(value) })),
              get: (v) => find(v)?.kind ?? "other",
              edit: (value) => ({ type: "update_account", id, kind: value as AccountKind }),
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
          h("td", {}, order, deleteButton(() => void ctx.apply({ type: "remove_account", id }))),
        );
      });

      const name = h("input", { type: "text", "aria-label": t("Name"), autocomplete: "off" });
      const kind = h(
        "select",
        { "aria-label": t("Type") },
        ...KINDS.map((value) => h("option", { value }, kindLabel(value))),
      );
      const form = h(
        "form",
        { class: "add-row" },
        name,
        kind,
        h("button", { type: "submit" }, t("Add an account")),
      );
      form.addEventListener("submit", (event) => {
        event.preventDefault();
        void ctx
          .apply({ type: "add_account", name: name.value, kind: kind.value as AccountKind })
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
                h("th", {}, t("Type")),
                h("th", {}, t("Sellable")),
                h("th", {}, t("Rounding unit")),
                h("th", {}, t("Can buy")),
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
