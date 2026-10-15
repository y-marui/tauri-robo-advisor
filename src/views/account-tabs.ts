import { getAccountBalance } from "../api.ts";
import { h } from "../dom.ts";
import { t } from "../i18n.ts";
import type { Context, Panel } from "../panel.ts";
import type { AccountBalance, PortfolioView } from "../types.ts";
import { categoryName, deltaClass, signed, suggestionBox, yen } from "./suggestion.ts";

// The right panel: one tab per account. The upper part compares the products
// inside the account with their weights; the lower part suggests how to spend
// an addition in this account. Every figure comes from Rust.

export function accountTabsPanel(_ctx: Context): Panel {
  const tabs = h("div", { class: "tabs", role: "tablist", "aria-label": t("By account") });
  const balance = h("div", { class: "account-balance" });
  const body = h(
    "div",
    { role: "tabpanel", class: "tab-body" },
    h("h3", {}, t("Balance inside the account")),
    balance,
    h("h3", {}, t("Add to this account")),
  );

  let current: PortfolioView | null = null;
  let selected: number | null = null;
  let tabSignature: string | null = null;
  let sequence = 0;
  let shownAccount: number | null = null;

  const box = suggestionBox({
    modes: ["fixed_add", "buy_only_to_target"],
    account: () => selected,
    noAccountMessage: () => t("Choose an account first."),
  });
  body.append(box.el);

  const el = h("section", { class: "card account-tabs" }, h("h2", {}, t("By account")), tabs, body);

  const productName = (view: PortfolioView, id: number): string =>
    view.portfolio.products.find((p) => p.id === id)?.name ?? "?";

  function balanceView(view: PortfolioView, data: AccountBalance): HTMLElement {
    const rows: HTMLElement[] = [];
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
            h("th", { scope: "row" }, productName(view, product.product)),
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
      return h("p", { class: "note" }, t("Nothing held or buyable in this account."));
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
    const account = selected;
    if (account === null) {
      shownAccount = null;
      balance.replaceChildren();
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

  function select(id: number): void {
    selected = id;
    for (const tab of tabs.querySelectorAll<HTMLButtonElement>("[role=tab]")) {
      const active = tab.dataset["account"] === String(id);
      tab.setAttribute("aria-selected", String(active));
      tab.tabIndex = active ? 0 : -1;
    }
    if (current) {
      box.update(current);
      refreshBalance(current);
    }
  }

  function buildTabs(view: PortfolioView): void {
    tabs.replaceChildren(
      ...view.portfolio.accounts.map((account) => {
        const tab = h(
          "button",
          { type: "button", role: "tab", "data-account": String(account.id), class: "tab" },
          account.name,
        );
        tab.addEventListener("click", () => select(account.id));
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
      body.hidden = accounts.length === 0;
      const signature = accounts.map((a) => `${a.id}:${a.name}`).join(",");
      if (signature !== tabSignature) {
        tabSignature = signature;
        buildTabs(view);
        if (selected === null || !accounts.some((a) => a.id === selected)) {
          selected = accounts[0]?.id ?? null;
        }
        if (accounts.length === 0) {
          tabs.replaceChildren(h("p", { class: "note" }, t("No accounts yet.")));
        }
      }
      if (selected !== null) {
        select(selected);
      }
    },
  };
}
