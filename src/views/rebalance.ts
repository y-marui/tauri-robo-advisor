import { getAccountBalance } from "../api.ts";
import { h } from "../dom.ts";
import { t } from "../i18n.ts";
import type { Context, Panel } from "../panel.ts";
import { accountFor, type Selection } from "../rebalance-account.ts";
import type { AccountBalance, PortfolioView } from "../types.ts";
import { categoryName, deltaClass, signed, suggestionBox, yen } from "./suggestion.ts";

// The "Rebalance" sub-tab of the holdings area: one tab per account, led by "All". Every tab has the
// same layout: the suggestion form and its result on top, the account's holdings (balance inside
// the account) below. An account tab works on its account; "All" works on the rebalancing account,
// which it also lets the person choose. Products the account does not hold are left out of both
// the list and the suggestion. Every figure comes from Rust.

export function rebalancePanel(ctx: Context): Panel {
  const tabs = h("div", { class: "tabs", role: "tablist", "aria-label": t("Rebalance") });
  const balance = h("div", { class: "account-balance" });

  let current: PortfolioView | null = null;
  let selected: Selection = "all";
  let tabSignature: string | null = null;
  let sequence = 0;
  let shownAccount: number | null = null;

  const accountNow = (): number | null =>
    accountFor(selected, current?.portfolio.settings.target_account ?? null);

  // The same form for "All" and the account tabs; "All" works on the rebalancing account.
  const box = suggestionBox({
    modes: ["fixed_add", "buy_only_to_target", "full_rebalance", "fixed_sell"],
    account: accountNow,
    heldOnly: true,
    noAccountMessage: () =>
      selected === "all" ? t("Choose the rebalancing account first.") : t("Choose an account first."),
  });
  const targetSelect = h("select", { "aria-label": t("Rebalancing account") });
  // The suggestion must use the saved account, so it waits while the choice is being saved.
  targetSelect.addEventListener("change", () => {
    box.setBusy(true);
    void ctx
      .apply({
        type: "set_target_account",
        account: targetSelect.value === "" ? null : Number(targetSelect.value),
      })
      .finally(() => box.setBusy(false));
  });
  let targetSignature: string | null = null;
  const targetField = h("label", { class: "field" }, h("span", {}, t("Rebalancing account")), targetSelect);
  const body = h(
    "div",
    { role: "tabpanel", class: "tab-body" },
    targetField,
    box.el,
    h("h3", {}, t("Balance inside the account")),
    balance,
  );

  const el = h("section", { class: "card rebalance" }, h("h2", {}, t("Rebalance")), tabs, body);

  const productName = (view: PortfolioView, id: number): string =>
    view.portfolio.products.find((p) => p.id === id)?.name ?? "?";

  function balanceView(view: PortfolioView, data: AccountBalance): HTMLElement {
    const rows: HTMLElement[] = [];
    const account = accountNow();
    const excluded = view.portfolio.accounts.find((a) => a.id === account)?.excluded ?? [];
    for (const category of data.categories) {
      if (category.products.length === 0) {
        continue;
      }
      rows.push(
        h(
          "tr",
          { class: "group" },
          h("th", { scope: "rowgroup" }, categoryName(view, category.category)),
          h("td", { class: "num" }, yen(category.total)),
          h("td", {}),
          h("td", {}),
        ),
      );
      for (const product of category.products) {
        rows.push(
          h(
            "tr",
            {},
            h(
              "th",
              { scope: "row" },
              productName(view, product.product),
              excluded.includes(product.product)
                ? h("span", { class: "note", title: t("Left out of trading in this account") }, ` (${t("Excluded")})`)
                : "",
            ),
            h("td", { class: "num" }, yen(product.current)),
            h("td", { class: "num" }, product.target === null ? "" : yen(product.target)),
            h(
              "td",
              { class: `num ${product.shortfall === null ? "" : deltaClass(product.shortfall)}` },
              product.shortfall === null ? "" : signed(product.shortfall),
            ),
          ),
        );
      }
    }
    if (rows.length === 0) {
      return h("p", { class: "note" }, t("This account holds nothing yet. Add a record in Settings."));
    }
    return h(
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
            h("th", {}, t("Product")),
            h("th", {}, t("Current")),
            h("th", {}, t("Target")),
            h("th", {}, t("Short (+) / over (-)")),
          ),
        ),
        h("tbody", {}, ...rows),
      ),
    );
  }

  function refreshBalance(view: PortfolioView): void {
    const account = accountNow();
    if (account === null) {
      shownAccount = null;
      sequence += 1;
      balance.replaceChildren(
        h("p", { class: "note" }, selected === "all" ? t("Choose the rebalancing account first.") : ""),
      );
      return;
    }
    const mine = ++sequence;
    if (shownAccount !== account) {
      // Never show one account's figures under another account's tab.
      balance.replaceChildren(h("p", { class: "note" }, t("Loading…")));
    }
    getAccountBalance(account)
      .then((data) => {
        if (mine === sequence) {
          shownAccount = account;
          balance.replaceChildren(balanceView(view, data));
        }
      })
      .catch((error) => {
        if (mine === sequence) {
          shownAccount = null;
          balance.replaceChildren(
            h("p", { class: "note error" }, t("Could not load: {reason}", { reason: String(error) })),
          );
        }
      });
  }

  function select(id: Selection): void {
    selected = id;
    for (const tab of tabs.querySelectorAll<HTMLButtonElement>("[role=tab]")) {
      const active = tab.dataset["account"] === String(id);
      tab.setAttribute("aria-selected", String(active));
      tab.tabIndex = active ? 0 : -1;
    }
    targetField.hidden = id !== "all";
    if (current) {
      box.update(current);
      refreshBalance(current);
    }
  }

  function buildTabs(view: PortfolioView): void {
    const entries: { id: Selection; name: string }[] = [
      { id: "all", name: t("All") },
      ...view.portfolio.accounts.map((a) => ({ id: a.id, name: a.name })),
    ];
    tabs.replaceChildren(
      ...entries.map((entry) => {
        const tab = h(
          "button",
          { type: "button", role: "tab", "data-account": String(entry.id), class: "tab" },
          entry.name,
        );
        tab.addEventListener("click", () => select(entry.id));
        tab.addEventListener("keydown", (event) => {
          const all = [...tabs.querySelectorAll<HTMLButtonElement>("[role=tab]")];
          const index = all.indexOf(tab);
          const step = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0;
          if (step !== 0) {
            const next = all[(index + step + all.length) % all.length];
            next?.focus();
            next?.click();
            event.preventDefault();
          }
        });
        return tab;
      }),
    );
  }

  return {
    el,
    render(view) {
      current = view;
      const accounts = view.portfolio.accounts;
      const signature = accounts.map((a) => `${a.id}:${a.name}`).join(",");
      if (signature !== tabSignature) {
        tabSignature = signature;
        buildTabs(view);
        if (selected !== "all" && !accounts.some((a) => a.id === selected)) {
          selected = "all";
        }
      }
      if (signature !== targetSignature) {
        targetSignature = signature;
        targetSelect.replaceChildren(
          h("option", { value: "" }, t("None")),
          ...accounts.map((a) => h("option", { value: String(a.id) }, a.name)),
        );
      }
      if (document.activeElement !== targetSelect) {
        targetSelect.value = String(view.portfolio.settings.target_account ?? "");
      }
      select(selected);
    },
  };
}
