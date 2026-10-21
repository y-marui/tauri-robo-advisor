// Which account a Rebalance tab works on: an account tab uses its own account, and "All" uses the
// rebalancing (target) account, so both tabs show the same layout for one account.

export type Selection = number | "all";

export function accountFor(selected: Selection, target: number | null): number | null {
  return selected === "all" ? target : selected;
}
