import { getOverview, suggestRebalance } from "../api.ts";
import { cleanDecimal, formatAmount, fractionToPercent } from "../decimal.ts";
import { h } from "../dom.ts";
import { getLanguage, t } from "../i18n.ts";
import type { Context, Panel } from "../panel.ts";
import type {
  Note,
  Overview,
  PortfolioView,
  SuggestMode,
  Suggestion,
} from "../types.ts";

// The balance charts and the suggestion box. Every figure comes from Rust.
// Numbers are converted with `Number` only to place shapes on the screen; the
// text shown to the user is always the exact decimal string from Rust.

const SVG_NS = "http://www.w3.org/2000/svg";
const PALETTE_SIZE = 6;

function svg<K extends keyof SVGElementTagNameMap>(
  tag: K,
  attrs: Record<string, string | number> = {},
  ...children: SVGElement[]
): SVGElementTagNameMap[K] {
  const element = document.createElementNS(SVG_NS, tag);
  for (const [name, value] of Object.entries(attrs)) {
    element.setAttribute(name, String(value));
  }
  element.append(...children);
  return element;
}

const percent = (fraction: string): string => `${fractionToPercent(fraction)}%`;
const yen = (amount: string): string => formatAmount(amount, getLanguage());

/** "+1,000" or "-1,000"; the sign carries the meaning, not only the color. */
function signed(amount: string): string {
  const text = yen(amount);
  return text.startsWith("-") || amount === "0" ? text : `+${text}`;
}

function deltaClass(amount: string): string {
  return amount.startsWith("-") ? "delta-sell" : amount === "0" ? "" : "delta-buy";
}

function categoryName(view: PortfolioView, id: number): string {
  return view.portfolio.categories.find((c) => c.id === id)?.name ?? "?";
}

/** Target and current share of each category, as two horizontal bars. */
function barChart(view: PortfolioView, overview: Overview): SVGElement {
  const rows = overview.categories;
  const peak = Math.max(0.01, ...rows.flatMap((r) => [Number(r.current_share), Number(r.target_share)]));
  const left = 0;
  const width = 300;
  const rowHeight = 44;
  const barHeight = 10;
  const chart = svg("svg", {
    viewBox: `0 0 ${width} ${rows.length * rowHeight}`,
    class: "chart",
    role: "img",
    "aria-label": t("Target vs current"),
  });
  rows.forEach((row, index) => {
    const top = index * rowHeight;
    const label = svg("text", { x: left, y: top + 12, class: "chart-label" });
    label.textContent = categoryName(view, row.category);
    chart.append(label);
    const bars: [string, string, number][] = [
      [row.target_share, "bar-target", top + 18],
      [row.current_share, `bar-current swatch-${index % PALETTE_SIZE}`, top + 18 + barHeight + 2],
    ];
    for (const [share, className, y] of bars) {
      const length = (Number(share) / peak) * (width - 90);
      chart.append(
        svg("rect", { x: left, y, width: Math.max(length, 0), height: barHeight, class: className }),
      );
      const value = svg("text", { x: left + length + 6, y: y + barHeight - 1, class: "chart-value" });
      value.textContent = percent(share);
      chart.append(value);
    }
  });
  return chart;
}

/** The share of each category in the whole, as a pie. */
function pieChart(overview: Overview): SVGElement {
  const slices = overview.categories
    .map((c, index) => ({ share: Number(c.current_share), index }))
    .filter((s) => s.share > 0);
  const sum = slices.reduce((acc, s) => acc + s.share, 0);
  const chart = svg("svg", {
    viewBox: "-50 -50 100 100",
    class: "chart pie",
    role: "img",
    "aria-label": t("Composition"),
  });
  if (slices.length === 1) {
    chart.append(svg("circle", { r: 48, class: `swatch-${(slices[0]?.index ?? 0) % PALETTE_SIZE}` }));
    return chart;
  }
  let angle = -Math.PI / 2;
  for (const slice of slices) {
    const next = angle + (slice.share / sum) * 2 * Math.PI;
    const large = next - angle > Math.PI ? 1 : 0;
    const point = (a: number) => `${(48 * Math.cos(a)).toFixed(3)} ${(48 * Math.sin(a)).toFixed(3)}`;
    chart.append(
      svg("path", {
        d: `M 0 0 L ${point(angle)} A 48 48 0 ${large} 1 ${point(next)} Z`,
        class: `swatch-${slice.index % PALETTE_SIZE}`,
      }),
    );
    angle = next;
  }
  return chart;
}

function legend(view: PortfolioView, overview: Overview): HTMLElement {
  return h(
    "ul",
    { class: "legend" },
    ...overview.categories.map((row, index) =>
      h(
        "li",
        {},
        h("span", { class: `dot swatch-${index % PALETTE_SIZE}`, "aria-hidden": "true" }),
        `${categoryName(view, row.category)} ${percent(row.current_share)} / ${percent(row.target_share)}`,
      ),
    ),
  );
}

function describeNote(note: Note, view: PortfolioView): string {
  switch (note.type) {
    case "amount_rounded":
      return t("The amount was lowered from {requested} to {applied} JPY to fit the rounding unit.", {
        requested: yen(note.requested),
        applied: yen(note.applied),
      });
    case "category_not_filled":
      return t("“{category}” is {shortfall} JPY short of its target.", {
        category: categoryName(view, note.category),
        shortfall: yen(note.shortfall),
      });
    case "already_balanced":
      return t("Every category is already within tolerance.");
  }
}

function suggestionView(view: PortfolioView, suggestion: Suggestion): HTMLElement {
  const productName = (id: number) => view.portfolio.products.find((p) => p.id === id)?.name ?? "?";
  const accountName =
    view.portfolio.accounts.find((a) => a.id === suggestion.account)?.name ?? "?";
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
  const productRows = suggestion.products.map((line) =>
    h(
      "tr",
      {},
      h("th", { scope: "row" }, productName(line.product)),
      h("td", { class: `num ${deltaClass(line.delta)}` }, signed(line.delta)),
    ),
  );

  return h(
    "div",
    { class: "suggestion" },
    h("p", { class: "summary" }, summary),
    ...suggestion.notes.map((n) => h("p", { class: "note" }, describeNote(n, view))),
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
            h("th", {}, t("Category")),
            h("th", {}, t("Current")),
            h("th", {}, t("Change")),
            h("th", {}, t("After")),
          ),
        ),
        h("tbody", {}, ...categoryRows),
      ),
    ),
    productRows.length === 0
      ? null
      : h(
          "div",
          { class: "table-wrap" },
          h("h3", {}, t("Trades in {account}", { account: accountName })),
          h(
            "table",
            {},
            h("thead", {}, h("tr", {}, h("th", {}, t("Product")), h("th", {}, t("Change")))),
            h("tbody", {}, ...productRows),
          ),
        ),
    h("p", { class: "note" }, t("Taxes are not taken into account in suggestions.")),
  );
}

type ModeType = SuggestMode["type"];

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

export function overviewPanel(ctx: Context): Panel {
  const charts = h("div", { class: "charts" });
  const result = h("div", { class: "suggestion-result", "aria-live": "polite" });
  const message = h("p", { class: "note", role: "status" });

  const targetSelect = h("select", { "aria-label": t("Rebalancing account") });
  targetSelect.addEventListener("change", () => {
    void ctx.apply({
      type: "set_target_account",
      account: targetSelect.value === "" ? null : Number(targetSelect.value),
    });
  });

  const modeSelect = h(
    "select",
    { "aria-label": t("Suggestion mode") },
    ...MODES.map((m) => h("option", { value: m.value }, m.label())),
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

  const syncAmount = (): void => {
    const needs = MODES.find((m) => m.value === modeSelect.value)?.needsAmount ?? false;
    amount.hidden = !needs;
  };
  modeSelect.addEventListener("change", syncAmount);
  syncAmount();

  let current: PortfolioView | null = null;
  let accountSignature: string | null = null;
  let sequence = 0; // suggestions
  let chartSequence = 0;

  const showError = (error: unknown): void => {
    result.replaceChildren();
    message.textContent = t("Could not calculate: {reason}", { reason: String(error) });
    message.classList.add("error");
  };

  run.addEventListener("click", () => {
    const view = current;
    const account = view?.portfolio.settings.target_account ?? null;
    if (!view || account === null) {
      message.classList.add("error");
      message.textContent = t("Choose the rebalancing account first.");
      return;
    }
    const type = modeSelect.value as ModeType;
    let mode: SuggestMode;
    if (type === "fixed_add" || type === "fixed_sell") {
      const value = cleanDecimal(amount.value);
      if (value === null || value.startsWith("-") || value === "0") {
        message.classList.add("error");
        message.textContent = t("Enter a positive amount.");
        return;
      }
      mode = { type, amount: value };
    } else {
      mode = { type };
    }
    const mine = ++sequence;
    run.disabled = true;
    message.classList.remove("error");
    message.textContent = t("Calculating…");
    suggestRebalance({ account, mode })
      .then((suggestion) => {
        if (mine !== sequence) {
          return;
        }
        message.textContent = "";
        result.replaceChildren(suggestionView(view, suggestion));
      })
      .catch((error) => {
        if (mine === sequence) {
          showError(error);
        }
      })
      .finally(() => {
        run.disabled = false;
      });
  });

  const controls = h(
    "div",
    { class: "suggest-controls" },
    h("label", { class: "field" }, h("span", {}, t("Rebalancing account")), targetSelect),
    h("label", { class: "field" }, h("span", {}, t("Suggestion mode")), modeSelect),
    h("div", { class: "add-row" }, amount, run),
  );

  const el = h(
    "section",
    { class: "card overview" },
    h("h2", {}, t("Overall balance")),
    charts,
    h("h2", {}, t("Suggestion")),
    controls,
    message,
    result,
  );

  return {
    el,
    render(view) {
      current = view;
      // The numbers behind an earlier suggestion may have changed.
      sequence += 1;
      result.replaceChildren();
      message.textContent = "";
      message.classList.remove("error");

      const signature = view.portfolio.accounts.map((a) => `${a.id}:${a.name}`).join(",");
      if (signature !== accountSignature) {
        accountSignature = signature;
        targetSelect.replaceChildren(
          h("option", { value: "" }, t("None")),
          ...view.portfolio.accounts.map((a) => h("option", { value: String(a.id) }, a.name)),
        );
      }
      const target = String(view.portfolio.settings.target_account ?? "");
      if (document.activeElement !== targetSelect) {
        targetSelect.value = target;
      }

      const mine = ++chartSequence;
      getOverview()
        .then((overview) => {
          if (mine !== chartSequence) {
            return;
          }
          if (overview.categories.length === 0 || Number(overview.total) === 0) {
            charts.replaceChildren(h("p", { class: "note" }, t("No holdings yet.")));
            return;
          }
          charts.replaceChildren(
            h("h3", {}, t("Target vs current")),
            h(
              "p",
              { class: "muted key" },
              h("span", { class: "dot bar-target", "aria-hidden": "true" }),
              t("Target"),
              h("span", { class: "dot bar-current swatch-0", "aria-hidden": "true" }),
              t("Current"),
            ),
            barChart(view, overview),
            h("h3", {}, t("Composition")),
            h("div", { class: "pie-row" }, pieChart(overview), legend(view, overview)),
            h("p", { class: "muted" }, `${t("Total")}: ${yen(overview.total)}`),
          );
        })
        .catch((error) => {
          if (mine === chartSequence) {
            charts.replaceChildren(
              h("p", { class: "note error" }, t("Could not load: {reason}", { reason: String(error) })),
            );
          }
        });
    },
  };
}
