// Rebalance: which (account, product) pairs take part in a suggestion. The data is
// the per-account exclusion list (`Account.excluded`): a product takes part when it is not excluded.

/** The exclusion list after every shown product takes part. Products that are not shown keep their state. */
export function selectAll(excluded: number[], shown: number[]): number[] {
  return excluded.filter((id) => !shown.includes(id));
}

/** The exclusion list after none of the shown products takes part. */
export function clearAll(excluded: number[], shown: number[]): number[] {
  return [...new Set([...excluded, ...shown])];
}

/** The exclusion list after one product is switched. */
export function toggle(excluded: number[], product: number, takesPart: boolean): number[] {
  const rest = excluded.filter((id) => id !== product);
  return takesPart ? rest : [...rest, product];
}
