import { invoke } from "@tauri-apps/api/core";

import type { Edit, PortfolioView } from "./types.ts";

/** The current portfolio and the problems that block suggestions. */
export function getPortfolio(): Promise<PortfolioView> {
  return invoke<PortfolioView>("get_portfolio");
}

/**
 * Sends one edit. Rust validates it, saves it, and returns the new state; a
 * rejected edit rejects this promise with a message and changes nothing.
 */
export function applyEdit(edit: Edit): Promise<PortfolioView> {
  return invoke<PortfolioView>("apply_portfolio_edit", { edit });
}
