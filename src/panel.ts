import { h } from "./dom.ts";
import { t, type MessageKey } from "./i18n.ts";
import type { Edit, Overview, PortfolioView } from "./types.ts";

/** What the views need from the app. */
export interface Context {
  /** Sends an edit; the app shows the result and refreshes every panel. Resolves to whether it was accepted. */
  apply(edit: Edit): Promise<boolean>;
  /** Shows a message without sending anything. */
  fail(message: string): void;
  /** Takes a new state that did not come from an edit (for example after the data folder changed). */
  setView(view: PortfolioView, message: string): void;
}

/** Copies the current state into a control, unless the user is typing in it. */
export type Bind = (view: PortfolioView) => void;

export interface Panel {
  el: HTMLElement;
  render(view: PortfolioView): void;
  /** The JPY valuation for `view`, fetched once per refresh; arrives after `render`. */
  showOverview?(view: PortfolioView, overview: Overview): void;
  /** The valuation could not be computed. */
  overviewFailed?(error: unknown): void;
}

/**
 * A titled card. The body is rebuilt only when `signature` changes (rows added
 * or removed, options changed); otherwise the existing controls are just
 * updated in place, so focus and tab order survive every saved edit.
 */
export function panel(
  title: MessageKey,
  signature: (view: PortfolioView) => string,
  build: (view: PortfolioView, bind: (binding: Bind) => void) => Node[],
): Panel {
  const body = h("div", { class: "card-body" });
  const el = h("section", { class: "card" }, h("h2", {}, t(title)), body);
  let lastSignature: string | null = null;
  let bindings: Bind[] = [];
  return {
    el,
    render(view) {
      const next = signature(view);
      if (next !== lastSignature) {
        lastSignature = next;
        bindings = [];
        body.replaceChildren(...build(view, (binding) => bindings.push(binding)));
      }
      for (const binding of bindings) {
        binding(view);
      }
    },
  };
}
