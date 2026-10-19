import { h } from "../dom.ts";
import { t } from "../i18n.ts";
import type { Context, Panel } from "../panel.ts";
import { suggestionBox } from "./suggestion.ts";

// The "Suggestions" sub-tab of the holdings area: the overall rebalancing suggestion for the
// rebalancing account. It lives in the wide main area, so its tables have room. Every figure
// comes from Rust.

export function suggestionsPanel(ctx: Context): Panel {
  let accountSignature: string | null = null;
  let current: Parameters<Panel["render"]>[0] | null = null;
  const box = suggestionBox({
    modes: ["fixed_add", "buy_only_to_target", "full_rebalance", "fixed_sell"],
    account: () => current?.portfolio.settings.target_account ?? null,
    noAccountMessage: () => t("Choose the rebalancing account first."),
  });

  const targetSelect = h("select", { "aria-label": t("Rebalancing account") });
  // The suggestion must use the saved account, so it waits while the choice is
  // being saved.
  targetSelect.addEventListener("change", () => {
    box.setBusy(true);
    void ctx
      .apply({
        type: "set_target_account",
        account: targetSelect.value === "" ? null : Number(targetSelect.value),
      })
      .finally(() => box.setBusy(false));
  });

  const el = h(
    "section",
    { class: "card suggestions" },
    h("h2", {}, t("Suggestion")),
    h("label", { class: "field" }, h("span", {}, t("Rebalancing account")), targetSelect),
    box.el,
  );

  return {
    el,
    render(view) {
      current = view;
      box.update(view);
      const signature = view.portfolio.accounts.map((a) => `${a.id}:${a.name}`).join(",");
      if (signature !== accountSignature) {
        accountSignature = signature;
        targetSelect.replaceChildren(
          h("option", { value: "" }, t("None")),
          ...view.portfolio.accounts.map((a) => h("option", { value: String(a.id) }, a.name)),
        );
      }
      if (document.activeElement !== targetSelect) {
        targetSelect.value = String(view.portfolio.settings.target_account ?? "");
      }
    },
  };
}
