import { productGroups, slotKey, slotTrade, tradesBySlot, unlistedTrades, type ProductRow } from "../account-products.ts";
import { getAccountBalance } from "../api.ts";
import { h } from "../dom.ts";
import { cleanDecimal } from "../decimal.ts";
import { amountField, checkboxField } from "../fields.ts";
import { t } from "../i18n.ts";
import type { Bind, Context, Panel } from "../panel.ts";
import { loadListMode, loadMethod, saveListMode, saveMethod, type ListMode } from "../rebalance-choices.ts";
import { clearAll, selectAll, toggle } from "../rebalance-selection.ts";
import { weightOf } from "../product-shares.ts";
import type { Account, AccountBalance, PortfolioView, ProductLine } from "../types.ts";
import { typeLabel } from "./labels.ts";
import { categoryName, deltaClass, modeLabel, signed, suggestionBox, yen } from "./suggestion.ts";

// The "Rebalance" sub-tab of the holdings area: one tab per account, led by "All". Both kinds of tab
// have the suggestion form and its result on top. An account tab trades in its account; "All" trades
// across every account. Below the form, an account tab shows the account's product list and "All"
// shows one table of every account's list with an account column. A list holds only the products
// the account holds. Each row has a checkbox "takes part in trading" (the account's exclusion list,
// shown the other way round); the list can also show just the ticked ones. The account's "sell allowed" flag and rounding unit are edited here too. Buying a
// product the account does not hold is still not suggested. Every figure comes from Rust.

type Selection = number | "all";

export function rebalancePanel(ctx: Context): Panel {
  const tabs = h("div", { class: "tabs", role: "tablist", "aria-label": t("Rebalance") });

  let current: PortfolioView | null = null;
  let selected: Selection = "all";
  let tabSignature: string | null = null;
  let settingsSignature: string | null = null;
  // Per account: the figures last received, and whether the list shows every product.
  const balances = new Map<number, AccountBalance>();
  const listModes = new Map<Selection, ListMode>();
  let sequence = 0;

  const accountNow = (): number | null => (selected === "all" ? null : selected);

  // The same form for "All" and the account tabs; "All" trades in every account.
  const box = suggestionBox({
    modes: ["fixed_add", "buy_only_to_target", "full_rebalance", "fixed_sell"],
    account: accountNow,
    heldOnly: true,
  });

  // The account's own settings, kept between renders so typing in the field is not interrupted.
  box.onModeChange(() => {
    if (current) {
      renderLists(current);
    }
  });

  // The lists show what the suggestion changes, so they follow the result.
  box.onResult(() => {
    if (current) {
      renderLists(current);
    }
  });

  box.onMethodChange((method) => saveMethod(selected, method));

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
  const excludedOf = (view: PortfolioView, id: number): number[] => {
    const account = accountOf(view, id);
    return (selected === "all" ? account?.excluded_all : account?.excluded) ?? [];
  };
  const save = (changes: [number, number[]][]): void => {
    saving = true;
    holdControls();
    box.setBusy(true);
    // One account after the other: each change starts from the list as the previous one saved it.
    void changes
      .reduce(
        (done, [id, products]) => done.then(() => ctx.apply({ type: "set_excluded", id, products, all: selected === "all" })),
        Promise.resolve() as Promise<unknown>,
      )
      .finally(() => {
        saving = false;
        holdControls();
        box.setBusy(false);
      });
  };

  const productName = (view: PortfolioView, id: number): string =>
    view.portfolio.products.find((p) => p.id === id)?.name ?? "?";

  const head = (withBox: boolean, withShare: boolean, withAccount: boolean, withTrade: boolean): HTMLElement =>
    h(
      "thead",
      {},
      h(
        "tr",
        {},
        withBox ? h("th", {}, t("Takes part")) : "",
        withAccount ? h("th", {}, t("Account")) : "",
        h("th", {}, t("Product")),
        withShare ? h("th", {}, t("Weight")) : "",
        h("th", {}, t("Current")),
        withTrade ? h("th", {}, t("Change")) : "",
        withTrade ? h("th", {}, t("After")) : "",
      ),
    );

  // The weight of a product for the mode chosen in the suggestion form: one setting per mode and
  // product for each tab: an account's tab has its own, and "All" has its own.
  function shareCell(view: PortfolioView, product: number): HTMLElement {
    const mode = box.mode();
    const saved = weightOf(view, mode, product, accountNow());
    const weight = h("input", {
      type: "text",
      inputmode: "decimal",
      class: "number",
      autocomplete: "off",
      "aria-label": t("{product}: weight ({mode})", { product: productName(view, product), mode: modeLabel(mode) }),
      value: saved,
    });
    weight.addEventListener("change", () => {
      const value = cleanDecimal(weight.value);
      if (value === null || value.startsWith("-") || Number(value) === 0) {
        ctx.fail(t("Enter a positive weight."));
        weight.value = saved;
        return;
      }
      void ctx.apply({ type: "set_product_share", mode, product, account: accountNow(), weight: value });
    });
    return h("td", { class: "share-cell" }, weight);
  }

  function productRow(
    view: PortfolioView,
    account: Account,
    row: ProductRow,
    withBox: boolean,
    withShare: boolean,
    withAccount: boolean,
    trades: Map<string, ProductLine[]> | null,
  ): HTMLElement[] {
    let cell: HTMLElement | string = "";
    if (withBox) {
      const check = h("input", {
        type: "checkbox",
        "aria-label": t("{product} takes part in trading", { product: productName(view, row.product) }),
      });
      check.checked = row.takesPart;
      check.addEventListener("change", () =>
        save([[account.id, toggle(excludedOf(current ?? view, account.id), row.product, check.checked)]]),
      );
      cell = h("td", { class: "center" }, check);
    }
    const trade = trades === null ? null : slotTrade(trades.get(slotKey(account.id, row.product)), row.types);
    const first = h(
      "tr",
      {},
      cell,
      withAccount ? h("td", {}, account.name) : "",
      h("th", { scope: "row" }, productName(view, row.product)),
      withShare ? shareCell(view, row.product) : "",
      h("td", { class: "num" }, yen(row.current)),
      ...tradeCells(trade?.inline ?? null, trades !== null, (trade?.parts.length ?? 0) > 0, row),
    );
    // A sale taken from several record types: one sub-row per type, with that type's own figures.
    const parts = (trade?.parts ?? []).map(({ line, current }) =>
      h(
        "tr",
        { class: "type-part" },
        withBox ? h("td", {}) : "",
        withAccount ? h("td", {}) : "",
        h(
          "td",
          { class: "type-label" },
          line.account_type ? typeLabel(line.account_type, view.portfolio.custom_types) : "",
        ),
        withShare ? h("td", {}) : "",
        h("td", { class: "num" }, yen(current)),
        h("td", { class: `num ${deltaClass(line.delta)}` }, signed(line.delta)),
        h("td", { class: "num" }, yen(line.after)),
      ),
    );
    return [first, ...parts];
  }

  // The "Change" and "After" cells of a row, when a suggestion is shown. A row without a trade stays
  // as it is: no change, and the same value after. A row whose sale is shown per type leaves them
  // empty: its sub-rows carry the figures.
  function tradeCells(line: ProductLine | null, shown: boolean, split: boolean, row: ProductRow): HTMLElement[] {
    if (!shown) {
      return [];
    }
    if (split) {
      return [h("td", {}), h("td", {})];
    }
    return [
      h("td", { class: `num ${line ? deltaClass(line.delta) : ""}` }, line ? signed(line.delta) : ""),
      h("td", { class: "num" }, line ? yen(line.after) : yen(row.current)),
    ];
  }

  // The product list of the tab: one account's, or on "All" every account's in one table with an
  // account column. Only products an account holds are candidates for it.
  function productList(view: PortfolioView, accounts: Account[]): HTMLElement {
    const multi = selected === "all";
    const mode = listModes.get(selected) ?? loadListMode(selected) ?? "all";
    const showAll = mode === "all";
    // A strategy account has its own ratios, so the shares of the portfolio do not apply to it, and
    // it takes no part in All.
    const single = multi ? undefined : accounts[0];
    const withShare = single?.strategy?.type !== "products";
    const listed = multi ? accounts.filter((a) => a.strategy?.type !== "products") : accounts;

    // Category -> its rows, each with its account. A product strategy has the category `null`.
    const merged = new Map<number | null, { total: string | null; rows: { account: Account; row: ProductRow }[] }>();
    for (const account of listed) {
      for (const group of productGroups(view, account, balances.get(account.id) ?? null, showAll, multi)) {
        const entry = merged.get(group.category) ?? { total: multi ? null : group.total, rows: [] };
        entry.rows.push(...group.rows.map((row) => ({ account, row })));
        merged.set(group.category, entry);
      }
    }
    const order = [null, ...view.portfolio.categories.map((c) => c.id)].filter((c) => merged.has(c));
    const productOrder = new Map(view.portfolio.products.map((p, i) => [p.id, i]));
    const accountOrder = new Map(view.portfolio.accounts.map((a, i) => [a.id, i]));
    const shown = new Map<number, number[]>();
    for (const group of merged.values()) {
      group.rows.sort(
        (x, y) =>
          (productOrder.get(x.row.product) ?? 0) - (productOrder.get(y.row.product) ?? 0) ||
          (accountOrder.get(x.account.id) ?? 0) - (accountOrder.get(y.account.id) ?? 0),
      );
      for (const { account, row } of group.rows) {
        shown.set(account.id, [...(shown.get(account.id) ?? []), row.product]);
      }
    }

    const toggleButton = h(
      "button",
      { type: "button", class: "link-button", "aria-pressed": String(!showAll) },
      showAll ? t("Tradable only") : t("Show all"),
    );
    toggleButton.addEventListener("click", () => {
      const next: ListMode = showAll ? "tradable" : "all";
      listModes.set(selected, next);
      saveListMode(selected, next);
      if (current) {
        renderLists(current);
      }
    });
    const controls = h("div", { class: "button-row" }, toggleButton);
    if (showAll) {
      const changeAll = (edit: (excluded: number[], shownIds: number[]) => number[]): void =>
        save(
          [...shown].map(([id, products]) => [id, edit(excludedOf(current ?? view, id), products)]),
        );
      const selectAllButton = h("button", { type: "button", class: "link-button" }, t("Select all"));
      selectAllButton.addEventListener("click", () => changeAll(selectAll));
      const clearButton = h("button", { type: "button", class: "link-button" }, t("Clear"));
      clearButton.addEventListener("click", () => changeAll(clearAll));
      controls.append(selectAllButton, clearButton);
    }

    const suggestion = box.result();
    const trades = suggestion === null ? null : tradesBySlot(suggestion.products);
    const rows: HTMLElement[] = [];
    for (const category of order) {
      const group = merged.get(category);
      if (!group) {
        continue;
      }
      if (category !== null) {
        rows.push(
          h(
            "tr",
            { class: "group" },
            showAll ? h("td", {}) : "",
            multi ? h("td", {}) : "",
            h("th", { scope: "rowgroup" }, categoryName(view, category)),
            withShare ? h("td", {}) : "",
            h("td", { class: "num" }, group.total === null ? "" : yen(group.total)),
            trades === null ? "" : h("td", {}),
            trades === null ? "" : h("td", {}),
          ),
        );
      }
      for (const { account, row } of group.rows) {
        rows.push(...productRow(view, account, row, showAll, withShare, multi, trades));
      }
    }

    const strategyNote =
      single?.strategy?.type === "products"
        ? h(
            "p",
            { class: "note" },
            t("This account follows its own list of products and ratios. Category targets are not used, and products outside the list are neither bought nor sold."),
          )
        : "";
    const table =
      rows.length === 0
        ? h("p", { class: "note" }, showAll ? t("No products are held yet.") : t("No product takes part in trading."))
        : h("div", { class: "table-wrap" }, h("table", {}, head(showAll, withShare, multi, trades !== null), h("tbody", {}, ...rows)));
    // A trade whose product has no row here (not expected: trades are in held, listed products) is
    // still shown, below the table.
    const shownKeys = new Set([...shown].flatMap(([id, products]) => products.map((p) => slotKey(id, p))));
    const unlisted =
      suggestion === null
        ? []
        : unlistedTrades(
            suggestion.products,
            shownKeys,
            listed.map((a) => a.id),
          );
    const unlistedTable =
      unlisted.length === 0
        ? ""
        : h(
            "div",
            { class: "table-wrap" },
            h("h3", {}, t("Trades not listed in the table above")),
            h(
              "table",
              {},
              h(
                "thead",
                {},
                h("tr", {}, h("th", {}, t("Account")), h("th", {}, t("Product")), h("th", {}, t("Change")), h("th", {}, t("After"))),
              ),
              h(
                "tbody",
                {},
                ...unlisted.map((line) =>
                  h(
                    "tr",
                    {},
                    h("td", {}, accountOf(view, line.account)?.name ?? "?"),
                    h(
                      "th",
                      { scope: "row" },
                      line.account_type
                        ? `${productName(view, line.product)} (${typeLabel(line.account_type, view.portfolio.custom_types)})`
                        : productName(view, line.product),
                    ),
                    h("td", { class: `num ${deltaClass(line.delta)}` }, signed(line.delta)),
                    h("td", { class: "num" }, yen(line.after)),
                  ),
                ),
              ),
            ),
          );
    return h(
      "div",
      { class: "product-list" },
      controls,
      strategyNote,
      withShare
        ? h(
            "p",
            { class: "note" },
            t("The weight is set for the mode chosen above ({mode}); each mode keeps its own weight for a product, and every account tab and All keep their own. It splits a category's amount among its products, or the whole amount among all products when allocating by product weights.", {
              mode: modeLabel(box.mode()),
            }),
          )
        : "",
      table,
      unlistedTable,
      h(
        "p",
        { class: "note" },
        t("Only products the account holds are listed. Ticked products take part in trading in their account. All and each account keep their own ticks. Products you leave out are never bought or sold, but still count toward their category. Tradable only lists just the ticked products."),
      ),
    );
  }

  function renderLists(view: PortfolioView): void {
    const accounts = selected === "all" ? view.portfolio.accounts : view.portfolio.accounts.filter((a) => a.id === selected);
    settingsSlot.replaceChildren();
    if (selected === "all") {
      lists.replaceChildren(
        productList(view, accounts),
        h(
          "details",
          { class: "account-settings-all" },
          h("summary", {}, t("Account settings")),
          ...accounts.map((account) =>
            h("fieldset", { class: "participation-account" }, h("legend", {}, account.name), settingsRows.get(account.id) ?? ""),
          ),
        ),
      );
    } else {
      const only = accounts[0];
      const row = only ? settingsRows.get(only.id) : undefined;
      if (row) {
        settingsSlot.replaceChildren(row);
      }
      lists.replaceChildren(productList(view, accounts));
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
    box.setMethod(loadMethod(id) ?? "follow_portfolio");
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
