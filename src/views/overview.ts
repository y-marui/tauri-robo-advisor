import { fractionToPercent } from "../decimal.ts";
import { h } from "../dom.ts";
import { t } from "../i18n.ts";
import type { Context, Panel } from "../panel.ts";
import type { Overview, PortfolioView } from "../types.ts";
import { categoryName, suggestionBox, yen } from "./suggestion.ts";

// The balance charts and the overall suggestion. Every figure comes from Rust.
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

export function overviewPanel(ctx: Context): Panel {
  const charts = h("div", { class: "charts" });

  let current: PortfolioView | null = null;
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
    { class: "card overview" },
    h("h2", {}, t("Overall balance")),
    charts,
    h("h2", {}, t("Suggestion")),
    h("label", { class: "field" }, h("span", {}, t("Rebalancing account")), targetSelect),
    box.el,
  );

  let accountSignature: string | null = null;
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
      const target = String(view.portfolio.settings.target_account ?? "");
      if (document.activeElement !== targetSelect) {
        targetSelect.value = target;
      }
    },
    showOverview(view, overview) {
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
    },
    overviewFailed(error) {
      charts.replaceChildren(
        h("p", { class: "note error" }, t("Could not load: {reason}", { reason: String(error) })),
      );
    },
  };
}
