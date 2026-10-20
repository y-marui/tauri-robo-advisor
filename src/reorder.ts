// The order computation behind drag-and-drop reordering (no DOM here, so it can be tested).

/**
 * The place a dragged row ends up at when it is dropped into `slot` (the gap before row
 * `slot`, from 0 for the top to `count` for the end), or `null` when that is where it already is.
 * The result is what `Move*To` edits take as `index`.
 */
export function dropIndex(from: number, slot: number, count: number): number | null {
  if (from < 0 || from >= count || slot < 0 || slot > count) {
    return null;
  }
  const index = slot > from ? slot - 1 : slot;
  return index === from ? null : index;
}

/** The gap the pointer is in: before the first row whose middle is below `y`. */
export function slotAt(middles: number[], y: number): number {
  const index = middles.findIndex((middle) => y < middle);
  return index === -1 ? middles.length : index;
}
