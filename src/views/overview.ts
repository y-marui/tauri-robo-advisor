import { fractionToPercent } from "../decimal.ts";
import { h } from "../dom.ts";
import { t } from "../i18n.ts";
import type { Context, Panel } from "../panel.ts";
import type { Overview, PortfolioView } from "../types.ts";
import { groupEntries, groupName } from "./groups.ts";
import { categoryName } from "./suggestion.ts";

// The balance charts. Every figure comes from Rust.
// Numbers are converted with `Number` only to place shapes on the screen; the
// text shown to the user is always the exact decimal string from Rust.

const SVG_NS = "http://www.w3.org/2000/svg";
const PALETTE_SIZE = 6;

/** A category's fade within its group; never 1, so it never matches the group's own color. */
const categoryOpacity = (index: number): number => Math.max(0.4, 0.8 - 0.2 * index);

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

interface BarRow {
  label: string;
  target: string;
  current: string;
  className: string;
  opacity: number;
  group: boolean;
  /** Extra space above the row: the break between two groups. */
  gap: number;
}

/** The rows of the bar chart: with groups, each group's total first and then its categories. */
function barRows(view: PortfolioView, overview: Overview): BarRow[] {
  const entries = groupEntries(view);
  const share = (id: number) => overview.categories.find((c) => c.category === id);
  if (entries.length === 0) {
    return overview.categories.map((row, index) => ({
      label: categoryName(view, row.category),
      target: row.target_share,
      current: row.current_share,
      className: `swatch-${index % PALETTE_SIZE}`,
      opacity: 1,
      group: false,
      gap: 0,
    }));
  }
  const rows: BarRow[] = [];
  entries.forEach((entry, index) => {
    const className = entry.id === null ? "swatch-none" : `swatch-${index % PALETTE_SIZE}`;
    const total = overview.groups.find((g) => g.group === entry.id);
    rows.push({
      label: groupName(view, entry.id),
      target: total?.target_share ?? "0",
      current: total?.current_share ?? "0",
      className,
      opacity: 1,
      group: true,
      gap: index === 0 ? 0 : 20,
    });
    entry.categories.forEach((category, j) => {
      const row = share(category.id);
      rows.push({
        label: categoryName(view, category.id),
        target: row?.target_share ?? "0",
        current: row?.current_share ?? "0",
        className,
        opacity: categoryOpacity(j),
        group: false,
        gap: 0,
      });
    });
  });
  return rows;
}

/** Target and current share, as two horizontal bars each. */
function barChart(view: PortfolioView, overview: Overview): SVGElement {
  const rows = barRows(view, overview);
  const peak = Math.max(0.01, ...rows.flatMap((r) => [Number(r.current), Number(r.target)]));
  const left = 0;
  const width = 300;
  const rowHeight = 44;
  const barHeight = 10;
  const height = rows.reduce((acc, r) => acc + rowHeight + r.gap, 0);
  const chart = svg("svg", {
    viewBox: `0 0 ${width} ${height}`,
    class: "chart",
    role: "img",
    "aria-label": t("Target vs current"),
  });
  let top = 0;
  for (const row of rows) {
    top += row.gap;
    const label = svg("text", { x: left, y: top + 12, class: row.group ? "chart-label chart-group" : "chart-label" });
    label.textContent = row.label;
    chart.append(label);
    const bars: [string, string, number, number][] = [
      [row.target, "bar-target", top + 18, 1],
      [row.current, `bar-current ${row.className}`, top + 18 + barHeight + 2, row.opacity],
    ];
    for (const [value, className, y, opacity] of bars) {
      const length = (Number(value) / peak) * (width - 90);
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
      const text = svg("text", { x: left + length + 6, y: y + barHeight - 1, class: "chart-value" });
      text.textContent = percent(value);
      chart.append(text);
    }
    top += rowHeight;
  }
  return chart;
}

interface Slice {
  share: number;
  className: string;
  opacity: number;
  /** The name and the exact percentage, drawn on the chart. */
  name: string;
  text: string;
}

/** The angle at the middle of each visible slice, clockwise from the top. */
function sectors(slices: Slice[]): { slice: Slice; from: number; to: number }[] {
  const visible = slices.filter((s) => s.share > 0);
  const sum = visible.reduce((acc, s) => acc + s.share, 0);
  let angle = -Math.PI / 2;
  return visible.map((slice) => {
    const span = visible.length === 1 ? 2 * Math.PI - 0.0001 : (slice.share / sum) * 2 * Math.PI;
    const sector = { slice, from: angle, to: angle + span };
    angle += span;
    return sector;
  });
}

const point = (radius: number, a: number): string =>
  `${(radius * Math.cos(a)).toFixed(3)} ${(radius * Math.sin(a)).toFixed(3)}`;

/** One ring of sectors between two radii. A single full sector is drawn as two halves. */
function ring(slices: Slice[], inner: number, outer: number, fade = 1): SVGElement[] {
  return sectors(slices).map(({ slice, from, to }) => {
    const large = to - from > Math.PI ? 1 : 0;
    const d =
      `M ${point(outer, from)} A ${outer} ${outer} 0 ${large} 1 ${point(outer, to)} ` +
      `L ${point(inner, to)} A ${inner} ${inner} 0 ${large} 0 ${point(inner, from)} Z`;
    return svg("path", { d, class: slice.className, "fill-opacity": slice.opacity * fade });
  });
}

/** The name and percentage inside a sector, in white. Small sectors get none. */
function insideLabels(slices: Slice[], radius: number): SVGElement[] {
  return sectors(slices)
    .filter(({ from, to }) => to - from > 0.5)
    .map(({ slice, from, to }) => {
      const mid = (from + to) / 2;
      const text = svg("text", {
        x: (radius * Math.cos(mid)).toFixed(2),
        y: (radius * Math.sin(mid)).toFixed(2),
        class: "pie-inside",
        "text-anchor": "middle",
      });
      const name = svg("tspan", { x: (radius * Math.cos(mid)).toFixed(2), dy: "-0.1em" });
      name.textContent = slice.name;
      const value = svg("tspan", { x: (radius * Math.cos(mid)).toFixed(2), dy: "1.15em" });
      value.textContent = slice.text;
      text.append(name, value);
      return text;
    });
}

/** The labels stay within this distance of the center (the view is 76 high each way). */
const LABEL_LIMIT = 72;

/** Names outside the ring, each joined to its sector by a leader line. */
function outsideLabels(slices: Slice[], radius: number): SVGElement[] {
  const gap = 8;
  const items = sectors(slices).map(({ slice, from, to }) => {
    const mid = (from + to) / 2;
    return { slice, mid, right: Math.cos(mid) >= 0, y: (radius + 8) * Math.sin(mid) };
  });
  const shapes: SVGElement[] = [];
  for (const right of [true, false]) {
    const side = items.filter((i) => i.right === right).sort((a, b) => a.y - b.y);
    // Keep neighbouring labels from overlapping.
    side.forEach((item, i) => {
      if (i > 0) item.y = Math.max(item.y, side[i - 1].y + gap);
    });
    // Pull a crowded cluster back up so the last label stays inside the view, then settle the top.
    for (let i = side.length - 1; i >= 0; i--) {
      const limit = i === side.length - 1 ? LABEL_LIMIT : side[i + 1].y - gap;
      side[i].y = Math.min(side[i].y, limit);
    }
    side.forEach((item, i) => {
      item.y = Math.max(item.y, i === 0 ? -LABEL_LIMIT : side[i - 1].y + gap);
    });
    for (const item of side) {
      const edge = point(radius + 1, item.mid);
      const elbowX = (radius + 8) * Math.cos(item.mid);
      const endX = right ? radius + 14 : -(radius + 14);
      shapes.push(
        svg("polyline", {
          points: `${edge.replace(" ", ",")} ${elbowX.toFixed(2)},${item.y.toFixed(2)} ${endX},${item.y.toFixed(2)}`,
          class: "pie-leader",
        }),
      );
      const label = svg("text", {
        x: endX + (right ? 2 : -2),
        y: (item.y + 2).toFixed(2),
        class: "pie-outside",
        "text-anchor": right ? "start" : "end",
      });
      label.textContent = `${item.slice.name} ${item.slice.text}`;
      shapes.push(label);
    }
  }
  return shapes;
}

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
        name: categoryName(view, c.category),
        text: percent(c.current_share),
      })),
      groupSlices: [],
    };
  }
  const slices: Slice[] = [];
  const groupSlices: Slice[] = [];
  entries.forEach((entry, index) => {
    const className = entry.id === null ? "swatch-none" : `swatch-${index % PALETTE_SIZE}`;
    const row = overview.groups.find((g) => g.group === entry.id);
    groupSlices.push({
      share: Number(row?.current_share ?? 0),
      className,
      opacity: 1,
      name: groupName(view, entry.id),
      text: percent(row?.current_share ?? "0"),
    });
    entry.categories.forEach((category, j) => {
      slices.push({
        share: share(category.id),
        className,
        opacity: categoryOpacity(j),
        name: categoryName(view, category.id),
        text: percent(overview.categories.find((c) => c.category === category.id)?.current_share ?? "0"),
      });
    });
  });
  return { slices, groupSlices };
}

/** The composition as a doughnut: the groups inside as a pie (when there are any), the categories outside as a pale ring. */
function doughnutChart(view: PortfolioView, overview: Overview): SVGElement {
  const { slices, groupSlices } = categorySlices(view, overview);
  const chart = svg("svg", {
    viewBox: "-165 -76 330 152",
    class: "chart pie",
    role: "img",
    "aria-label": groupSlices.length > 0 ? t("Groups (inner) and categories (outer)") : t("Composition"),
  });
  if (groupSlices.length === 0) {
    chart.append(...ring(slices, 0, 58), ...insideLabels(slices, 36));
    return chart;
  }
  chart.append(
    ...ring(groupSlices, 0, 50),
    ...ring(slices, 53, 65),
    ...insideLabels(groupSlices, 33),
    ...outsideLabels(slices, 65),
  );
  return chart;
}

function chartPanel(title: string, draw: (view: PortfolioView, overview: Overview) => Element[]): Panel {
  const charts = h("div", { class: "charts" });
  const el = h("section", { class: "card overview" }, h("h2", {}, title), charts);
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
      charts.replaceChildren(...draw(view, overview));
    },
    overviewFailed(error) {
      charts.replaceChildren(
        h("p", { class: "note error" }, t("Could not load: {reason}", { reason: String(error) })),
      );
    },
  };
}

/** The composition as a doughnut. */
export function overviewPanel(_ctx: Context): Panel {
  return chartPanel(t("Overall balance"), (view, overview) => [doughnutChart(view, overview)]);
}

/** The target and the current share of each category, as bars. */
export function targetPanel(_ctx: Context): Panel {
  return chartPanel(t("Target vs current"), (view, overview) => [
    h(
      "p",
      { class: "muted key" },
      h("span", { class: "dot bar-target", "aria-hidden": "true" }),
      t("Target"),
      h("span", { class: "dot bar-current swatch-none", "aria-hidden": "true" }),
      t("Current"),
    ),
    barChart(view, overview),
  ]);
}
