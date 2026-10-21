import { suggestRebalance } from "../api.ts";
import { cleanDecimal, formatAmount } from "../decimal.ts";
import { h } from "../dom.ts";
import { getLanguage, t } from "../i18n.ts";
import type { Note, PortfolioView, SuggestMode, Suggestion } from "../types.ts";

// The suggestion box shared by the overall balance panel and the account tabs:
// a mode, an amount when the mode needs one, and the result of
// `suggest_rebalance`. Every figure comes from Rust.

export const yen = (amount: string): string => formatAmount(amount, getLanguage());

/** "+1,000" or "-1,000"; the sign carries the meaning, not only the color. */
export function signed(amount: string): string {
  const text = yen(amount);
  return text.startsWith("-") || amount === "0" ? text : `+${text}`;
}

export function deltaClass(amount: string): string {
  return amount.startsWith("-") ? "delta-sell" : amount === "0" ? "" : "delta-buy";
}

export function categoryName(view: PortfolioView, id: number): string {
  return view.portfolio.categories.find((c) => c.id === id)?.name ?? "?";
}

function describeNote(note: Note, view: PortfolioView): string {
  switch (note.type) {
    case "amount_rounded":
      return t("The amount was lowered from {requested} to {applied} JPY to fit the rounding unit.", {
        requested: yen(note.requested),
        applied: yen(note.applied),
      });
    case "category_not_filled": {
      // A full rebalance reports an unfilled sale as a negative shortfall.
      const surplus = note.shortfall.startsWith("-");
      return t(
        surplus
          ? "“{category}” is still {amount} JPY over its target."
          : "“{category}” is {amount} JPY short of its target.",
        {
          category: categoryName(view, note.category),
          amount: yen(surplus ? note.shortfall.slice(1) : note.shortfall),
        },
      );
    }
    case "product_not_filled": {
      const surplus = note.shortfall.startsWith("-");
      return t(
        surplus
          ? "“{product}” is still {amount} JPY over its share."
          : "“{product}” is {amount} JPY short of its share.",
        {
          product: view.portfolio.products.find((p) => p.id === note.product)?.name ?? "?",
          amount: yen(surplus ? note.shortfall.slice(1) : note.shortfall),
        },
      );
    }
    case "already_balanced":
      return t("Every category is already within tolerance.");
  }
}

function suggestionView(view: PortfolioView, suggestion: Suggestion): HTMLElement {
  const productName = (id: number) => view.portfolio.products.find((p) => p.id === id)?.name ?? "?";
  const summary =
    suggestion.required_total !== null
      ? t("Needed to reach the target: {amount} JPY", { amount: yen(suggestion.required_total) })
      : t("Applied: {amount} JPY", { amount: yen(suggestion.applied) });

  const categoryRows = suggestion.categories.map((line) =>
    h(
      "tr",
      {},
      h("th", { scope: "row" }, categoryName(view, line.category)),
      h("td", { class: "num" }, yen(line.current)),
      h("td", { class: `num ${deltaClass(line.delta)}` }, signed(line.delta)),
      h("td", { class: "num" }, yen(line.after)),
    ),
  );
  // The trades are grouped by account, in the order of the account list.
  const tradeTables = view.portfolio.accounts
    .map((account) => ({
      account,
      lines: suggestion.products.filter((line) => line.account === account.id),
    }))
    .filter((group) => group.lines.length > 0)
    .map(({ account, lines }) =>
      h(
        "div",
        { class: "table-wrap" },
        h("h3", {}, t("Trades in {account}", { account: account.name })),
        h(
          "table",
          {},
          h("thead", {}, h("tr", {}, h("th", {}, t("Product")), h("th", {}, t("Change")))),
          h(
            "tbody",
            {},
            ...lines.map((line) =>
              h(
                "tr",
                {},
                h("th", { scope: "row" }, productName(line.product)),
                h("td", { class: `num ${deltaClass(line.delta)}` }, signed(line.delta)),
              ),
            ),
          ),
        ),
      ),
    );

  return h(
    "div",
    { class: "suggestion" },
    h("p", { class: "summary" }, summary),
    ...suggestion.notes.map((n) => h("p", { class: "note" }, describeNote(n, view))),
    // An account with a product strategy has no categories: only its trades are shown.
    categoryRows.length === 0
      ? ""
      : h(
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
                h("th", {}, t("Category")),
                h("th", {}, t("Current")),
                h("th", {}, t("Change")),
                h("th", {}, t("After")),
              ),
            ),
            h("tbody", {}, ...categoryRows),
          ),
        ),
    ...tradeTables,
    h("p", { class: "note" }, t("Taxes are not taken into account in suggestions.")),
  );
}

export type ModeType = SuggestMode["type"];

const MODES: { value: ModeType; label: () => string; needsAmount: boolean }[] = [
  { value: "fixed_add", label: () => t("Add a fixed amount"), needsAmount: true },
  {
    value: "buy_only_to_target",
    label: () => t("Amount needed to reach the target without selling"),
    needsAmount: false,
  },
  { value: "full_rebalance", label: () => t("Full rebalance"), needsAmount: false },
  { value: "fixed_sell", label: () => t("Sell a fixed amount"), needsAmount: true },
];

export interface SuggestionBox {
  el: HTMLElement;
  /** Gives the box the current state and drops a result that may be out of date. */
  update(view: PortfolioView): void;
  /** Holds the Suggest button while something it depends on is being saved. */
  setBusy(busy: boolean): void;
}

interface SuggestionBoxOptions {
  modes: ModeType[];
  /** The account to calculate for, or `null` for every account. */
  account: () => number | null;
  /** Only buy what the account already holds (every Rebalance tab). */
  heldOnly?: boolean;
}

export function suggestionBox(options: SuggestionBoxOptions): SuggestionBox {
  const result = h("div", { class: "suggestion-result", "aria-live": "polite" });
  const message = h("p", { class: "note", role: "status" });
  const available = MODES.filter((m) => options.modes.includes(m.value));
  const modeSelect = h(
    "select",
    { "aria-label": t("Suggestion mode") },
    ...available.map((m) => h("option", { value: m.value }, m.label())),
  );
  const amount = h("input", {
    type: "text",
    inputmode: "decimal",
    class: "number",
    autocomplete: "off",
    "aria-label": t("Amount (JPY)"),
    placeholder: t("Amount (JPY)"),
  });
  const run = h("button", { type: "button" }, t("Suggest"));

  let busy = false;
  let calculating = false;
  const updateRunButton = (): void => {
    run.disabled = busy || calculating;
  };

  const syncAmount = (): void => {
    amount.hidden = !(available.find((m) => m.value === modeSelect.value)?.needsAmount ?? false);
  };
  modeSelect.addEventListener("change", syncAmount);
  syncAmount();

  let current: PortfolioView | null = null;
  let sequence = 0;

  const fail = (text: string): void => {
    result.replaceChildren();
    message.classList.add("error");
    message.textContent = text;
  };

  run.addEventListener("click", () => {
    const view = current;
    const account = options.account();
    if (!view) {
      return;
    }
    const type = modeSelect.value as ModeType;
    let mode: SuggestMode;
    if (type === "fixed_add" || type === "fixed_sell") {
      const value = cleanDecimal(amount.value);
      if (value === null || value.startsWith("-") || value === "0") {
        fail(t("Enter a positive amount."));
        return;
      }
      mode = { type, amount: value };
    } else {
      mode = { type };
    }
    const mine = ++sequence;
    calculating = true;
    updateRunButton();
    message.classList.remove("error");
    message.textContent = t("Calculating…");
    suggestRebalance({ account, mode, held_only: options.heldOnly === true })
      .then((suggestion) => {
        if (mine === sequence) {
          message.textContent = "";
          result.replaceChildren(suggestionView(view, suggestion));
        }
      })
      .catch((error) => {
        if (mine === sequence) {
          fail(t("Could not calculate: {reason}", { reason: String(error) }));
        }
      })
      .finally(() => {
        calculating = false;
        updateRunButton();
      });
  });

  const el = h(
    "div",
    { class: "suggest-controls" },
    h("label", { class: "field" }, h("span", {}, t("Suggestion mode")), modeSelect),
    h("div", { class: "add-row" }, amount, run),
    message,
    result,
  );

  return {
    el,
    update(view) {
      current = view;
      // The numbers behind an earlier suggestion may have changed.
      sequence += 1;
      result.replaceChildren();
      message.textContent = "";
      message.classList.remove("error");
    },
    setBusy(value) {
      busy = value;
      updateRunButton();
    },
  };
}
