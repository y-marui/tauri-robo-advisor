import { getAccountBalance } from "../api.ts";
import { h } from "../dom.ts";
import { t } from "../i18n.ts";
import type { Context, Panel } from "../panel.ts";
import { clearAll, heldProducts, selectAll, toggle } from "../rebalance-selection.ts";
import type { AccountBalance, PortfolioView } from "../types.ts";
import { categoryName, deltaClass, signed, suggestionBox, yen } from "./suggestion.ts";

// The "Rebalance" sub-tab of the holdings area: one tab per account, led by "All". Both kinds of tab
// have the suggestion form and its result on top. An account tab trades in its account and shows the
// account's balance below. "All" trades across every account instead and shows, per account, the
// products that take part (the account's exclusion list, shown the other way round: ticked = not
// excluded). Products an account does not hold are left out of the suggestion. Every figure comes
// from Rust.

type Selection = number | "all";

export function rebalancePanel(ctx: Context): Panel {
  const tabs = h("div", { class: "tabs", role: "tablist", "aria-label": t("Rebalance") });
  const balance = h("div", { class: "account-balance" });

  let current: PortfolioView | null = null;
  let selected: Selection = "all";
  let tabSignature: string | null = null;
  let sequence = 0;
  let shownAccount: number | null = null;

  const accountNow = (): number | null => (selected === "all" ? null : selected);

  // The same form for "All" and the account tabs; "All" trades in every account.
  const box = suggestionBox({
    modes: ["fixed_add", "buy_only_to_target", "full_rebalance", "fixed_sell"],
    account: accountNow,
    heldOnly: true,
  });

  // "All": which products of each account take part.
  const participation = h("div", { class: "participation" });
  const participationSection = h(
    "div",
    {},
    h("h3", {}, t("Products taking part")),
    h("p", { class: "note" }, t("Untick a product to keep it out of the suggestion in that account.")),
    participation,
  );
  const checks = new Map<string, HTMLInputElement>();
  let participationSignature: string | null = null;

  const balanceSection = h("div", {}, h("h3", {}, t("Balance inside the account")), balance);
  const body = h("div", { role: "tabpanel", class: "tab-body" }, box.el, participationSection, balanceSection);

  const el = h("section", { class: "card rebalance", "aria-label": t("Rebalance") }, tabs, body);

  const excludedOf = (view: PortfolioView, account: number): number[] =>
    view.portfolio.accounts.find((a) => a.id === account)?.excluded ?? [];

  // The suggestion must use the saved list, so it waits while a change is being saved.
  // The controls are held while saving: a second change would start from the list as it was before
  // the first one was saved and overwrite it.
  let saving = false;
  const holdControls = (): void => {
    for (const control of participation.querySelectorAll<HTMLInputElement | HTMLButtonElement>("input, button")) {
      control.disabled = saving;
    }
  };
  const save = (account: number, products: number[]): void => {
    saving = true;
    holdControls();
    box.setBusy(true);
    void ctx
      .apply({ type: "set_excluded", id: account, products })
      .finally(() => {
        saving = false;
        holdControls();
        box.setBusy(false);
      });
  };

  function buildParticipation(view: PortfolioView): void {
    checks.clear();
    const name = (id: number): string => view.portfolio.products.find((p) => p.id === id)?.name ?? "?";
    participation.replaceChildren(
      ...view.portfolio.accounts.map((account) => {
        const shown = heldProducts(view, account.id);
        const selectAllButton = h("button", { type: "button", class: "link-button" }, t("Select all"));
        selectAllButton.addEventListener("click", () =>
          save(account.id, selectAll(current ? excludedOf(current, account.id) : [], shown)),
        );
        const clearButton = h("button", { type: "button", class: "link-button" }, t("Clear"));
        clearButton.addEventListener("click", () =>
          save(account.id, clearAll(current ? excludedOf(current, account.id) : [], shown)),
        );
        const items = shown.map((product) => {
          const check = h("input", { type: "checkbox" });
          check.addEventListener("change", () =>
            save(account.id, toggle(current ? excludedOf(current, account.id) : [], product, check.checked)),
          );
          checks.set(`${account.id}:${product}`, check);
          return h("label", { class: "check" }, check, name(product));
        });
        return h(
          "fieldset",
          { class: "participation-account" },
          h("legend", {}, account.name),
          h("div", { class: "button-row" }, selectAllButton, clearButton),
          items.length === 0 ? h("p", { class: "note" }, t("This account holds nothing yet. Add a record in Settings.")) : h("div", {}, ...items),
        );
      }),
    );
    holdControls();
  }

  function refreshParticipation(view: PortfolioView): void {
    const signature = view.portfolio.accounts
      .map((a) => `${a.id}:${a.name}:${heldProducts(view, a.id).join("/")}`)
      .join(",") + view.portfolio.products.map((p) => `${p.id}:${p.name}`).join(",");
    if (signature !== participationSignature) {
      participationSignature = signature;
      buildParticipation(view);
    }
    for (const [key, check] of checks) {
      const [account, product] = key.split(":").map(Number);
      check.checked = !excludedOf(view, account ?? -1).includes(product ?? -1);
    }
  }

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
    participationSection.hidden = id !== "all";
    balanceSection.hidden = id === "all";
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
      refreshParticipation(view);
      select(selected);
    },
  };
}
