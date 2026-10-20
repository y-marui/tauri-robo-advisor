import { closeBrowser, getBrowserSettings, listBrowsers, openBrowser } from "../api.ts";
import { pickBrowser } from "../browser-defaults.ts";
import { h } from "../dom.ts";
import { t } from "../i18n.ts";
import type { BrowserKind } from "../types.ts";

// Choosing the browser and the profile folder, opening the browser, and closing it. Shared by the
// fetch recipes screen and the "Fetch in browser" section, so both start the same dedicated
// browser. The last browser and folder are kept by the app (in a local file) and preselected here.

function kindLabel(kind: BrowserKind): string {
  switch (kind) {
    case "chrome":
      return "Chrome";
    case "edge":
      return "Edge";
    case "brave":
      return "Brave";
  }
}

interface Options {
  info(text: string): void;
  fail(text: string): void;
  /** Called after the browser was started (or found running). */
  onOpened(): Promise<void> | void;
  /** Called after the browser was closed. */
  onClosed(): void;
}

export interface BrowserControls {
  el: HTMLElement;
}

export function browserControls(options: Options): BrowserControls {
  const { info, fail, onOpened, onClosed } = options;
  const browserSelect = h("select", { "aria-label": t("Browser") });
  const profileInput = h("input", {
    type: "text",
    autocomplete: "off",
    "aria-label": t("Profile folder"),
    placeholder: t("Profile folder (empty: inside the app's data folder)"),
  });
  const openButton = h("button", { type: "button" }, t("Open browser"));
  const closeButton = h("button", { type: "button" }, t("Close browser"));
  const row = h("div", { class: "add-row" }, browserSelect, profileInput, openButton, closeButton);
  row.hidden = true;
  const none = h("p", { class: "note" }, t("No Chrome, Edge, or Brave was found on this computer."));
  none.hidden = true;

  const run = async (button: HTMLButtonElement, work: () => Promise<void>): Promise<void> => {
    button.disabled = true;
    try {
      await work();
    } catch (error) {
      fail(String(error));
    } finally {
      button.disabled = false;
    }
  };

  openButton.addEventListener("click", () => {
    void run(openButton, async () => {
      info(t("Opening the browser…"));
      const folder = profileInput.value.trim();
      await openBrowser(browserSelect.value as BrowserKind, folder === "" ? null : folder);
      await onOpened();
    });
  });

  closeButton.addEventListener("click", () => {
    void run(closeButton, async () => {
      await closeBrowser();
      onClosed();
      info(t("The browser was closed."));
    });
  });

  Promise.all([listBrowsers(), getBrowserSettings().catch(() => ({ kind: null, profile: "" }))])
    .then(([found, saved]) => {
      const kinds = found.map((b) => b.kind);
      browserSelect.replaceChildren(...kinds.map((kind) => h("option", { value: kind }, kindLabel(kind))));
      const chosen = pickBrowser(kinds, saved.kind);
      if (chosen !== null) {
        browserSelect.value = chosen;
      }
      profileInput.value = saved.profile;
      row.hidden = kinds.length === 0;
      none.hidden = kinds.length !== 0;
    })
    .catch((error) => fail(String(error)));

  return { el: h("div", {}, none, row) };
}
