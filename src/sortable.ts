// Drag-and-drop reordering of table rows, shared by the accounts, groups and categories lists.
// It uses pointer events on a grip (HTML5 drag events are unreliable in some webviews). The
// up/down buttons stay as the keyboard fallback. The order itself is changed by a `Move*To` edit
// from the caller; this part only shows the drag and the moved row.

import { h } from "./dom.ts";
import { t } from "./i18n.ts";
import { dropIndex, slotAt } from "./reorder.ts";

/** The row that was moved last, marked after the list is rebuilt in its new order. */
let moved: { list: string; id: number } | null = null;

/** Marks a row to be highlighted for a moment once its list is rebuilt (the buttons use it too). */
export function markMoved(list: string, id: number): void {
  moved = { list, id };
}

const FLASH_MS = 1500;

/** The grip that starts a drag. */
export function grip(): HTMLElement {
  return h("span", { class: "grip", "aria-hidden": "true", title: t("Drag to reorder") }, "⋮⋮");
}

export interface Sortable {
  /** The name of the list (accounts, groups, categories). */
  list: string;
  tbody: HTMLElement;
  /** The ids of the rows, in the order shown. */
  ids: number[];
  /** Sends the move of `id` to the place `index`. */
  move(id: number, index: number): void;
}

/** Makes the rows of `tbody` (one per id, in order) draggable by their `.grip`. */
export function makeSortable({ list, tbody, ids, move }: Sortable): void {
  const rows = (): HTMLElement[] => Array.from(tbody.children) as HTMLElement[];

  if (moved?.list === list) {
    const row = rows()[ids.indexOf(moved.id)];
    moved = null;
    if (row) {
      row.classList.add("just-moved");
      window.setTimeout(() => row.classList.remove("just-moved"), FLASH_MS);
    }
  }

  rows().forEach((row, from) => {
    const handle = row.querySelector<HTMLElement>(".grip");
    const id = ids[from];
    if (!handle || id === undefined) {
      return;
    }
    handle.addEventListener("pointerdown", (event) => {
      if (event.button !== 0) {
        return;
      }
      event.preventDefault();
      handle.setPointerCapture(event.pointerId);
      const all = rows();
      let slot = from;
      row.classList.add("dragging");
      tbody.classList.add("sorting");

      const show = () => {
        all.forEach((r) => r.classList.remove("drop-before", "drop-after"));
        // A line on the top edge of the row at `slot`, or the bottom edge of the last row.
        if (slot < all.length) {
          all[slot]?.classList.add("drop-before");
        } else {
          all[all.length - 1]?.classList.add("drop-after");
        }
      };
      const finish = (commit: boolean) => {
        handle.removeEventListener("pointermove", onMove);
        handle.removeEventListener("pointerup", onUp);
        handle.removeEventListener("pointercancel", onCancel);
        window.removeEventListener("keydown", onKey);
        row.classList.remove("dragging");
        tbody.classList.remove("sorting");
        all.forEach((r) => r.classList.remove("drop-before", "drop-after"));
        const index = commit ? dropIndex(from, slot, all.length) : null;
        if (index !== null) {
          markMoved(list, id);
          move(id, index);
        }
      };
      const onMove = (e: PointerEvent) => {
        const middles = all.map((r) => {
          const box = r.getBoundingClientRect();
          return box.top + box.height / 2;
        });
        slot = slotAt(middles, e.clientY);
        show();
      };
      const onUp = () => finish(true);
      const onCancel = () => finish(false);
      const onKey = (e: KeyboardEvent) => {
        if (e.key === "Escape") {
          finish(false);
        }
      };
      handle.addEventListener("pointermove", onMove);
      handle.addEventListener("pointerup", onUp);
      handle.addEventListener("pointercancel", onCancel);
      window.addEventListener("keydown", onKey);
      show();
    });
  });
}
