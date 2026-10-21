import { cleanDecimal } from "../decimal.ts";
import { h } from "../dom.ts";
import { percentField, textField } from "../fields.ts";
import { t } from "../i18n.ts";
import { panel, type Context, type Panel } from "../panel.ts";
import type { PortfolioView } from "../types.ts";

function field(label: string, control: HTMLElement): HTMLElement {
  return h("label", { class: "field" }, h("span", {}, label), control);
}

export function settingsPanel(ctx: Context): Panel {
  return panel(
    "Settings",
    (view) => view.portfolio.accounts.map((a) => `${a.id}:${a.name}`).join(","),
    (view, bind) => {
      let current: PortfolioView = view;
      bind((v) => {
        current = v;
      });

      const rate = textField({
        ctx,
        bind,
        label: t("USD/JPY rate"),
        get: (v) => v.portfolio.settings.usd_jpy ?? "",
        edit: (text) => {
          if (text === "") {
            return { type: "set_exchange_rate", usd_jpy: null };
          }
          const clean = cleanDecimal(text);
          return clean === null ? null : { type: "set_exchange_rate", usd_jpy: clean };
        },
        attrs: { inputmode: "decimal", class: "number" },
      });

      const categoryTolerance = percentField({
        ctx,
        bind,
        label: t("Category tolerance (%)"),
        get: (v) => v.portfolio.settings.category_tolerance,
        save: (fraction) => ({
          type: "set_tolerances",
          category: fraction,
          product: current.portfolio.settings.product_tolerance,
        }),
      });
      const productTolerance = percentField({
        ctx,
        bind,
        label: t("Product tolerance (%)"),
        get: (v) => v.portfolio.settings.product_tolerance,
        save: (fraction) => ({
          type: "set_tolerances",
          category: current.portfolio.settings.category_tolerance,
          product: fraction,
        }),
      });

      return [
        h(
          "div",
          { class: "fields" },
          field(t("USD/JPY rate"), rate),
          field(t("Category tolerance (%)"), categoryTolerance),
          field(t("Product tolerance (%)"), productTolerance),
        ),
        h("p", { class: "note" }, t("Taxes are not taken into account in suggestions.")),
      ];
    },
  );
}
