import { h } from "../dom.ts";
import { deleteButton, orderButtons, textField } from "../fields.ts";
import { t } from "../i18n.ts";
import { grip, makeSortable } from "../sortable.ts";
import { panel, type Context, type Panel } from "../panel.ts";
import type { Category, PortfolioView } from "../types.ts";

/** The name shown for a group; `null` is the categories that belong to no group. */
export function groupName(view: PortfolioView, id: number | null): string {
  if (id === null) {
    return t("Ungrouped");
  }
  return view.portfolio.groups.find((g) => g.id === id)?.name ?? "?";
}

export interface GroupEntry {
  /** `null`: the ungrouped categories. */
  id: number | null;
  categories: Category[];
}

/**
 * The groups in order, each with its categories, then the ungrouped categories (only if there
 * are any). Empty when no group exists: the screens then show the categories alone.
 */
export function groupEntries(view: PortfolioView): GroupEntry[] {
  const { groups, categories } = view.portfolio;
  if (groups.length === 0) {
    return [];
  }
  const entries: GroupEntry[] = groups.map((g) => ({
    id: g.id,
    categories: categories.filter((c) => c.group === g.id),
  }));
  const ungrouped = categories.filter((c) => c.group === undefined || c.group === null);
  if (ungrouped.length > 0) {
    entries.push({ id: null, categories: ungrouped });
  }
  return entries;
}

export function groupsPanel(ctx: Context): Panel {
  return panel(
    "Groups",
    (view) => view.portfolio.groups.map((g) => g.id).join(","),
    (view, bind) => {
      const rows = view.portfolio.groups.map((group, position) => {
        const id = group.id;
        return h(
          "tr",
          {},
          h("td", { class: "grip-cell" }, grip()),
          h(
            "td",
            {},
            textField({
              ctx,
              bind,
              label: t("Group name"),
              get: (v) => v.portfolio.groups.find((g) => g.id === id)?.name ?? "",
              edit: (text) => ({ type: "rename_group", id, name: text }),
            }),
          ),
          h(
            "td",
            {},
            orderButtons(ctx, position, view.portfolio.groups.length, (up) => ({ type: "move_group", id, up }), {
              list: "groups",
              id,
            }),
            deleteButton(() => void ctx.apply({ type: "remove_group", id })),
          ),
        );
      });

      const tbody = h("tbody", {}, ...rows);
      makeSortable({
        list: "groups",
        tbody,
        ids: view.portfolio.groups.map((x) => x.id),
        move: (id, index) => void ctx.apply({ type: "move_group_to", id, index }),
      });

      // The add form is the last row of the table; its controls belong to this (empty) form.
      const formId = "add-group-form";
      const name = h("input", { type: "text", "aria-label": t("Group name"), autocomplete: "off", form: formId });
      const form = h("form", { id: formId });
      const addRow = h(
        "tfoot",
        {},
        h("tr", {}, h("td", {}), h("td", {}, name), h("td", { class: "add-cell" }, h("button", { type: "submit", form: formId }, t("Add a group")))),
      );
      form.addEventListener("submit", (event) => {
        event.preventDefault();
        void ctx.apply({ type: "add_group", name: name.value }).then((ok) => {
          if (ok) {
            name.value = "";
            name.focus();
          }
        });
      });

      return [
        h(
          "p",
          { class: "muted" },
          t(
            "Groups sit above the categories. A group has no target of its own: it adds up its categories’ targets. Deleting a group keeps its categories.",
          ),
        ),
        h(
          "div",
          { class: "table-wrap" },
          h("table", { class: "list-table" }, tbody, addRow),
        ),
        form,
      ];
    },
  );
}
