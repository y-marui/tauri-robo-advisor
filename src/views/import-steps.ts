import { h } from "../dom.ts";
import { t } from "../i18n.ts";
import { panel, type Context, type Panel } from "../panel.ts";

// The numbered flow of the preset import. The first two steps show whether they are done (they
// can be read from the saved data); the others point at the cards below, which do the work.

export function importStepsPanel(_ctx: Context): Panel {
  return panel(
    "Preset import: steps",
    (view) =>
      [
        view.portfolio.categories.length,
        view.portfolio.accounts.length,
        view.portfolio.recipes.length,
      ].join(":"),
    (view) => {
      const hasBase = view.portfolio.categories.length > 0 && view.portfolio.accounts.length > 0;
      const hasRecipe = view.portfolio.recipes.length > 0;
      const step = (done: boolean | null, text: string): HTMLElement =>
        h(
          "li",
          { class: done === true ? "step done" : "step" },
          h("span", { class: "step-state" }, done === true ? t("Done") : done === false ? t("To do") : ""),
          " ",
          text,
        );
      return [
        h(
          "ol",
          { class: "steps" },
          step(hasBase, t("Add a category and an account (Settings tab).")),
          step(hasRecipe, t("Choose a preset and an account, and add it (Fetch recipes card).")),
          step(null, t("Open the browser and the login pages, and log in yourself (Fetch recipes card).")),
          step(null, t("Fetch (Fetch recipes card).")),
          step(null, t("Link the names to products (one click creates a product), then apply.")),
        ),
        h("p", { class: "note" }, t("A table from a file, pasted text, or the browser, with your own columns, is in the Import a table card below.")),
      ];
    },
  );
}
