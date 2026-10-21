import { productGroups, type ProductRow } from "../account-products.ts";
import { getAccountBalance } from "../api.ts";
import { h } from "../dom.ts";
import { amountField, checkboxField } from "../fields.ts";
import { t } from "../i18n.ts";
import type { Bind, Context, Panel } from "../panel.ts";
import { clearAll, selectAll, toggle } from "../rebalance-selection.ts";
import type { Account, AccountBalance, PortfolioView } from "../types.ts";
import { categoryName, deltaClass, signed, suggestionBox, yen } from "./suggestion.ts";

// The "Rebalance" sub-tab of the holdings area: one tab per account, led by "All". Both kinds of tab
// have the suggestion form and its result on top. An account tab trades in its account; "All" trades
// across every account. Below the form, an account tab shows the account's product list and "All"
// shows the same list once per account. The list is one component: every product with a checkbox
// "takes part in trading" (the account's exclusion list, shown the other way round), or just the
// ticked ones. The account's "sell allowed" flag and rounding unit are edited here too. Buying a
// product the account does not hold is still not suggested. Every figure comes from Rust.

type Selection = number | "all";
type ListMode = "all" | "tradable";

export function rebalancePanel(ctx: Context): Panel {
  const tabs = h("div", { class: "tabs", role: "tablist", "aria-label": t("Rebalance") });

  let current: PortfolioView | null = null;
  let selected: Selection = "all";
  let tabSignature: string | null = null;
  let settingsSignature: string | null = null;
  // Per account: the figures last received, and whether the list shows every product.
  const balances = new Map<number, AccountBalance>();
  const listModes = new Map<number, ListMode>();
  let sequence = 0;

  const accountNow = (): number | null => (selected === "all" ? null : selected);

  // The same form for "All" and the account tabs; "All" trades in every account.
  const box = suggestionBox({
    modes: ["fixed_add", "buy_only_to_target", "full_rebalance", "fixed_sell"],
    account: accountNow,
    heldOnly: true,
  });

  // The account's own settings, kept between renders so typing in the field is not interrupted.
  const settingsRows = new Map<number, HTMLElement>();
  let settingsBinders: Bind[] = [];
  const settingsSlot = h("div", { class: "account-settings-slot" });
  const lists = h("div", { class: "product-lists" });

  const body = h("div", { role: "tabpanel", class: "tab-body" }, settingsSlot, box.el, lists);

  const el = h("section", { class: "card rebalance", "aria-label": t("Rebalance") }, tabs, body);

  const accountOf = (view: PortfolioView, id: number): Account | undefined =>
    view.portfolio.accounts.find((a) => a.id === id);

  function buildSettings(view: PortfolioView): void {
    settingsRows.clear();
    settingsBinders = [];
    const add = (binder: Bind): void => {
      settingsBinders.push(binder);
    };
    for (const account of view.portfolio.accounts) {
      const id = account.id;
      const find = (v: PortfolioView): Account | undefined => accountOf(v, id);
      const sellable = checkboxField({
        ctx,
        bind: add,
        label: t("Sell allowed"),
        get: (v) => find(v)?.sellable ?? false,
        edit: (checked) => ({ type: "update_account", id, sellable: checked }),
      });
      const unit = amountField({
        ctx,
        bind: add,
        label: t("Rounding unit"),
        get: (v) => find(v)?.rounding_unit ?? "",
        save: (value) => ({ type: "update_account", id, rounding_unit: value }),
      });
      settingsRows.set(
        id,
        h(
          "div",
          { class: "account-settings" },
          h("label", { class: "check" }, sellable, t("Sell allowed")),
          h("label", { class: "check" }, t("Rounding unit"), unit),
        ),
      );
    }
  }

  // The suggestion must use the saved list, so it waits while a change is being saved.
  // The controls are held while saving: a second change would start from the list as it was before
  // the first one was saved and overwrite it.
  let saving = false;
  const holdControls = (): void => {
    for (const control of lists.querySelectorAll<HTMLInputElement | HTMLButtonElement>("input, button")) {
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

  const productName = (view: PortfolioView, id: number): string =>
    view.portfolio.products.find((p) => p.id === id)?.name ?? "?";

  const head = (withBox: boolean): HTMLElement =>
    h(
      "thead",
      {},
      h(
        "tr",
        {},
        withBox ? h("th", {}, t("Takes part")) : "",
        h("th", {}, t("Product")),
        h("th", {}, t("Current")),
        h("th", {}, t("Target")),
        h("th", {}, t("Short (+) / over (-)")),
      ),
    );

  function productRow(view: PortfolioView, account: Account, row: ProductRow, withBox: boolean): HTMLElement {
    let cell: HTMLElement | string = "";
    if (withBox) {
      const check = h("input", {
        type: "checkbox",
        "aria-label": t("{product} takes part in trading", { product: productName(view, row.product) }),
      });
      check.checked = row.takesPart;
      check.addEventListener("change", () =>
        save(account.id, toggle(accountOf(current ?? view, account.id)?.excluded ?? [], row.product, check.checked)),
      );
      cell = h("td", { class: "center" }, check);
    }
    return h(
      "tr",
      {},
      cell,
      h("th", { scope: "row" }, productName(view, row.product)),
      h("td", { class: "num" }, yen(row.current)),
      h("td", { class: "num" }, row.target === null ? "" : yen(row.target)),
      h(
        "td",
        { class: `num ${row.shortfall === null ? "" : deltaClass(row.shortfall)}` },
        row.shortfall === null ? "" : signed(row.shortfall),
      ),
    );
  }

  // One account's product list. The same for an account tab and for each account on "All".
  function productList(view: PortfolioView, account: Account): HTMLElement {
    const mode = listModes.get(account.id) ?? "all";
    const showAll = mode === "all";
    const groups = productGroups(view, account, balances.get(account.id) ?? null, showAll);
    const shown = groups.flatMap((g) => g.rows.map((r) => r.product));

    const toggleButton = h(
      "button",
      { type: "button", class: "link-button", "aria-pressed": String(!showAll) },
      showAll ? t("Tradable only") : t("Show all"),
    );
    toggleButton.addEventListener("click", () => {
      listModes.set(account.id, showAll ? "tradable" : "all");
      if (current) {
        renderLists(current);
      }
    });
    const controls = h("div", { class: "button-row" }, toggleButton);
    if (showAll) {
      const selectAllButton = h("button", { type: "button", class: "link-button" }, t("Select all"));
      selectAllButton.addEventListener("click", () =>
        save(account.id, selectAll(accountOf(current ?? view, account.id)?.excluded ?? [], shown)),
      );
      const clearButton = h("button", { type: "button", class: "link-button" }, t("Clear"));
      clearButton.addEventListener("click", () =>
        save(account.id, clearAll(accountOf(current ?? view, account.id)?.excluded ?? [], shown)),
      );
      controls.append(selectAllButton, clearButton);
    }

    const rows: HTMLElement[] = [];
    for (const group of groups) {
      if (group.category !== null) {
        rows.push(
          h(
            "tr",
            { class: "group" },
            showAll ? h("td", {}) : "",
            h("th", { scope: "rowgroup" }, categoryName(view, group.category)),
            h("td", { class: "num" }, group.total === null ? "" : yen(group.total)),
            h("td", {}),
            h("td", {}),
          ),
        );
      }
      for (const row of group.rows) {
        rows.push(productRow(view, account, row, showAll));
      }
    }

    const strategyNote =
      account.strategy?.type === "products"
        ? h(
            "p",
            { class: "note" },
            selected === "all"
              ? t("This account has its own strategy (specified products with ratios). It takes no part in All; use its own tab.")
              : t("This account follows its own list of products and ratios. Category targets are not used, and products outside the list are neither bought nor sold."),
          )
        : "";
    const table =
      rows.length === 0
        ? h("p", { class: "note" }, showAll ? t("There are no products yet. Add one in Settings.") : t("No product takes part in trading in this account."))
        : h("div", { class: "table-wrap" }, h("table", {}, head(showAll), h("tbody", {}, ...rows)));
    return h(
      "div",
      { class: "product-list" },
      controls,
      strategyNote,
      table,
      h(
        "p",
        { class: "note" },
        t("Ticked products take part in trading in this account. Products you leave out are never bought or sold, but still count toward their category. A product the account does not hold is not bought. Tradable only lists just the ticked products."),
      ),
    );
  }

  function renderLists(view: PortfolioView): void {
    const accounts = selected === "all" ? view.portfolio.accounts : view.portfolio.accounts.filter((a) => a.id === selected);
    settingsSlot.replaceChildren();
    if (selected === "all") {
      lists.replaceChildren(
        ...accounts.map((account) =>
          h(
            "fieldset",
            { class: "participation-account" },
            h("legend", {}, account.name),
            settingsRows.get(account.id) ?? "",
            productList(view, account),
          ),
        ),
      );
    } else {
      const only = accounts[0];
      const row = only ? settingsRows.get(only.id) : undefined;
      if (row) {
        settingsSlot.replaceChildren(row);
      }
      lists.replaceChildren(...accounts.map((account) => productList(view, account)));
    }
    holdControls();
  }

  // Fetches the figures of the accounts shown; a list is drawn at once from what is known and again
  // when the figures arrive.
  function refreshLists(view: PortfolioView): void {
    const mine = ++sequence;
    renderLists(view);
    const wanted = selected === "all" ? view.portfolio.accounts.map((a) => a.id) : [selected];
    for (const id of wanted) {
      getAccountBalance(id)
        .then((data) => {
          if (mine === sequence) {
            balances.set(id, data);
            renderLists(view);
          }
        })
        .catch(() => {
          // Without figures the list still shows the products and their checkboxes.
          if (mine === sequence) {
            balances.delete(id);
            renderLists(view);
          }
        });
    }
  }

  function select(id: Selection): void {
    selected = id;
    for (const tab of tabs.querySelectorAll<HTMLButtonElement>("[role=tab]")) {
      const active = tab.dataset["account"] === String(id);
      tab.setAttribute("aria-selected", String(active));
      tab.tabIndex = active ? 0 : -1;
    }
    if (current) {
      box.update(current);
      refreshLists(current);
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
      if (signature !== settingsSignature) {
        settingsSignature = signature;
        buildSettings(view);
      }
      for (const binder of settingsBinders) {
        binder(view);
      }
      select(selected);
    },
  };
}
