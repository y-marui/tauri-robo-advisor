import { fractionToPercent } from "../decimal.ts";
import { h } from "../dom.ts";
import { t } from "../i18n.ts";
import type { Context, Panel } from "../panel.ts";
import type { Overview, PortfolioView } from "../types.ts";
import { groupEntries, groupName } from "./groups.ts";
import { categoryName, yen } from "./suggestion.ts";

// The balance charts. Every figure comes from Rust.
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

/** The color class and fade of each category, the same in the bars, the doughnut, and the legend. */
function categoryColors(view: PortfolioView, overview: Overview): Map<number, { className: string; opacity: number }> {
  const colors = new Map<number, { className: string; opacity: number }>();
  const entries = groupEntries(view);
  if (entries.length === 0) {
    overview.categories.forEach((row, index) => {
      colors.set(row.category, { className: `swatch-${index % PALETTE_SIZE}`, opacity: 1 });
    });
    return colors;
  }
  entries.forEach((entry, index) => {
    const className = entry.id === null ? "swatch-none" : `swatch-${index % PALETTE_SIZE}`;
    entry.categories.forEach((category, j) => {
      colors.set(category.id, { className, opacity: Math.max(0.45, 1 - 0.2 * j) });
    });
  });
  return colors;
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
  const colors = categoryColors(view, overview);
  rows.forEach((row, index) => {
    const top = index * rowHeight;
    const color = colors.get(row.category) ?? { className: `swatch-${index % PALETTE_SIZE}`, opacity: 1 };
    const label = svg("text", { x: left, y: top + 12, class: "chart-label" });
    label.textContent = categoryName(view, row.category);
    chart.append(label);
    const bars: [string, string, number, number][] = [
      [row.target_share, "bar-target", top + 18, 1],
      [row.current_share, `bar-current ${color.className}`, top + 18 + barHeight + 2, color.opacity],
    ];
    for (const [share, className, y, opacity] of bars) {
      const length = (Number(share) / peak) * (width - 90);
      chart.append(
        svg("rect", {
          x: left,
          y,
          width: Math.max(length, 0),
          height: barHeight,
          class: className,
          "fill-opacity": opacity,
        }),
      );
      const value = svg("text", { x: left + length + 6, y: y + barHeight - 1, class: "chart-value" });
      value.textContent = percent(share);
      chart.append(value);
    }
  });
  return chart;
}

interface Slice {
  share: number;
  className: string;
  opacity: number;
}

/** One ring of sectors between two radii. A single full sector is drawn as two halves. */
function ring(slices: Slice[], inner: number, outer: number): SVGElement[] {
  const sum = slices.reduce((acc, s) => acc + s.share, 0);
  const shapes: SVGElement[] = [];
  const point = (radius: number, a: number) =>
    `${(radius * Math.cos(a)).toFixed(3)} ${(radius * Math.sin(a)).toFixed(3)}`;
  let angle = -Math.PI / 2;
  const visible = slices.filter((s) => s.share > 0);
  for (const slice of visible) {
    const span = visible.length === 1 ? 2 * Math.PI - 0.0001 : (slice.share / sum) * 2 * Math.PI;
    const next = angle + span;
    const large = span > Math.PI ? 1 : 0;
    const d =
      `M ${point(outer, angle)} A ${outer} ${outer} 0 ${large} 1 ${point(outer, next)} ` +
      `L ${point(inner, next)} A ${inner} ${inner} 0 ${large} 0 ${point(inner, angle)} Z`;
    shapes.push(svg("path", { d, class: slice.className, "fill-opacity": slice.opacity }));
    angle = next;
  }
  return shapes;
}

/** A dot's opacity as a class (inline styles are not allowed by the CSP). */
const shadeClass = (opacity: number): string => (opacity >= 1 ? "" : `shade-${Math.round((1 - opacity) * 5)}`);

/** The colors of the categories: those of a group share its color, in fading shades. */
function categorySlices(view: PortfolioView, overview: Overview): { slices: Slice[]; groupSlices: Slice[] } {
  const share = (id: number) =>
    Number(overview.categories.find((c) => c.category === id)?.current_share ?? 0);
  const entries = groupEntries(view);
  if (entries.length === 0) {
    return {
      slices: overview.categories.map((c, index) => ({
        share: Number(c.current_share),
        className: `swatch-${index % PALETTE_SIZE}`,
        opacity: 1,
      })),
      groupSlices: [],
    };
  }
  const slices: Slice[] = [];
  const groupSlices: Slice[] = [];
  entries.forEach((entry, index) => {
    const className = entry.id === null ? "swatch-none" : `swatch-${index % PALETTE_SIZE}`;
    const row = overview.groups.find((g) => g.group === entry.id);
    groupSlices.push({ share: Number(row?.current_share ?? 0), className, opacity: 1 });
    entry.categories.forEach((category, j) => {
      slices.push({ share: share(category.id), className, opacity: Math.max(0.45, 1 - 0.2 * j) });
    });
  });
  return { slices, groupSlices };
}

/** The composition as a doughnut: the groups inside (when there are any), the categories outside. */
function doughnutChart(view: PortfolioView, overview: Overview): SVGElement {
  const { slices, groupSlices } = categorySlices(view, overview);
  const chart = svg("svg", {
    viewBox: "-50 -50 100 100",
    class: "chart pie",
    role: "img",
    "aria-label": groupSlices.length > 0 ? t("Groups (inner) and categories (outer)") : t("Composition"),
  });
  if (groupSlices.length === 0) {
    chart.append(...ring(slices, 0, 48));
    return chart;
  }
  chart.append(...ring(groupSlices, 22, 33), ...ring(slices, 35, 48));
  return chart;
}

function legend(view: PortfolioView, overview: Overview): HTMLElement {
  const categoryItem = (id: number, className: string, opacity: number): HTMLElement => {
    const row = overview.categories.find((c) => c.category === id);
    return h(
      "li",
      {},
      h("span", { class: `dot ${className} ${shadeClass(opacity)}`, "aria-hidden": "true" }),
      `${categoryName(view, id)} ${row ? `${percent(row.current_share)} / ${percent(row.target_share)}` : ""}`,
    );
  };
  const entries = groupEntries(view);
  if (entries.length === 0) {
    return h(
      "ul",
      { class: "legend" },
      ...overview.categories.map((row, index) => categoryItem(row.category, `swatch-${index % PALETTE_SIZE}`, 1)),
    );
  }
  return h(
    "ul",
    { class: "legend" },
    ...entries.map((entry, index) => {
      const className = entry.id === null ? "swatch-none" : `swatch-${index % PALETTE_SIZE}`;
      const row = overview.groups.find((g) => g.group === entry.id);
      return h(
        "li",
        { class: "legend-group" },
        h("span", { class: `dot ${className}`, "aria-hidden": "true" }),
        h(
          "strong",
          {},
          `${groupName(view, entry.id)} ${row ? `${percent(row.current_share)} / ${percent(row.target_share)}` : ""}`,
        ),
        h(
          "ul",
          { class: "legend" },
          ...entry.categories.map((c, j) => categoryItem(c.id, className, Math.max(0.45, 1 - 0.2 * j))),
        ),
      );
    }),
  );
}

export function overviewPanel(_ctx: Context): Panel {
  const charts = h("div", { class: "charts" });

  const el = h("section", { class: "card overview" }, h("h2", {}, t("Overall balance")), charts);

  return {
    el,
    render() {
      // Nothing to show until the valuation arrives.
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
          h("span", { class: "dot bar-current swatch-none", "aria-hidden": "true" }),
          t("Current"),
        ),
        barChart(view, overview),
        h("h3", {}, t("Composition")),
        h("div", { class: "pie-row" }, doughnutChart(view, overview), legend(view, overview)),
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
