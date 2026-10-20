import {
  deleteBitflyerCredentials,
  fetchBitflyerHoldings,
  fetchBitflyerHoldingsWithPagePrices,
  hasBitflyerCredentials,
  openBitflyerPricePage,
  planImport,
  saveBitflyerCredentials,
} from "../api.ts";
import { formatAmount } from "../decimal.ts";
import { h } from "../dom.ts";
import { deleteButton } from "../fields.ts";
import { getLanguage, t } from "../i18n.ts";
import type { Context, Panel } from "../panel.ts";
import type { BitflyerHoldings, ImportPlan, ImportRow, PortfolioView } from "../types.ts";
import { importPlanView } from "./import-plan.ts";

// bitFlyer: the API key, and the balances. The key is typed here, handed to
// the OS keychain by Rust, and never shown again. Balances are fetched
// read-only, linked to products through aliases the user confirms (a name is
// never guessed), and only then written over the chosen account's holdings.

const SOURCE = "bitflyer";
const CURRENCY = "JPY";

export function bitflyerPanel(ctx: Context): Panel {
  const credentialStatus = h("p", { class: "note", role: "status" });
  const keyInput = h("input", {
    type: "password",
    autocomplete: "off",
    "aria-label": t("API key"),
    placeholder: t("API key"),
  });
  const secretInput = h("input", {
    type: "password",
    autocomplete: "off",
    "aria-label": t("API secret"),
    placeholder: t("API secret"),
  });
  const saveButton = h("button", { type: "button" }, t("Save key"));
  const removeKey = deleteButton(() => {
    removeKey.disabled = true;
    deleteBitflyerCredentials()
      .then(refreshCredentials)
      .catch((error) => ctx.fail(String(error)))
      .finally(() => {
        removeKey.disabled = false;
      });
  });
  removeKey.hidden = true;

  async function refreshCredentials(): Promise<void> {
    try {
      const saved = await hasBitflyerCredentials();
      credentialStatus.textContent = saved
        ? t("A key is saved in the system keychain. It cannot be shown again.")
        : t("No key is saved.");
      removeKey.hidden = !saved;
    } catch (error) {
      credentialStatus.textContent = String(error);
    }
  }

  saveButton.addEventListener("click", () => {
    const key = keyInput.value.trim();
    const secret = secretInput.value.trim();
    if (key === "" || secret === "") {
      ctx.fail(t("Enter both the API key and the secret."));
      return;
    }
    saveButton.disabled = true;
    saveBitflyerCredentials(key, secret)
      .then(() => {
        // The secret does not stay in the page once it is saved.
        keyInput.value = "";
        secretInput.value = "";
        return refreshCredentials();
      })
      .catch((error) => ctx.fail(String(error)))
      .finally(() => {
        saveButton.disabled = false;
      });
  });

  // ---- balances ------------------------------------------------------------

  const accountSelect = h("select", { "aria-label": t("Account to update") });
  const fetchButton = h("button", { type: "button" }, t("Fetch balances"));
  // One price basis for every currency: either the Lightning last price (a currency with no
  // market is left out), or the price page, whose table the person loaded with the browser.
  const basisSelect = h(
    "select",
    { "aria-label": t("Prices") },
    h("option", { value: "api" }, t("Prices: Lightning last price")),
    h("option", { value: "page" }, t("Prices: bitFlyer price page")),
  );
  const openPageButton = h("button", { type: "button" }, t("Open the price page"));
  const pageStatus = h("span", { class: "note", role: "status" });
  const pageNote = h(
    "div",
    { class: "add-row" },
    openPageButton,
    h(
      "p",
      { class: "note" },
      t(
        "Open the price page, wait until the prices are shown, then fetch the balances: the prices are read from that page. The app only shows the page in the browser and reads it; it does not connect to it.",
      ),
    ),
    pageStatus,
  );
  pageNote.hidden = true;
  basisSelect.addEventListener("change", () => {
    pageNote.hidden = basisSelect.value !== "page";
  });
  openPageButton.addEventListener("click", () => {
    openPageButton.disabled = true;
    pageStatus.textContent = t("Opening…");
    openBitflyerPricePage()
      .then(() => {
        pageStatus.textContent = t("Opened. Wait until the prices are shown, then fetch the balances.");
      })
      .catch((error) => {
        pageStatus.textContent = t("Could not open the page: {reason}", { reason: String(error) });
      })
      .finally(() => {
        openPageButton.disabled = false;
      });
  });
  const result = h("div", { class: "bitflyer-result", "aria-live": "polite" });

  let current: PortfolioView | null = null;
  let rows: ImportRow[] | null = null;
  let unpriced: string[] = [];
  let compared: BitflyerHoldings["holdings"] = [];
  let usedPage = false;
  let sequence = 0;
  let accountSignature: string | null = null;

  // The confirmation and the note name the selected account, so redraw them.
  accountSelect.addEventListener("change", () => {
    void showPlan().catch((error) =>
      result.replaceChildren(h("p", { class: "note error" }, String(error))),
    );
  });

  fetchButton.addEventListener("click", () => {
    fetchButton.disabled = true;
    result.replaceChildren(h("p", { class: "note" }, t("Fetching…")));
    const page = basisSelect.value === "page";
    (page ? fetchBitflyerHoldingsWithPagePrices() : fetchBitflyerHoldings())
      .then((fetched) => {
        usedPage = page;
        rows = fetched.holdings.map((x) => ({
          name: x.symbol,
          value: x.value,
          quantity: x.amount,
          price: x.price ?? null,
        }));
        compared = fetched.holdings.filter((x) => x.api_price != null);
        unpriced = fetched.unpriced;
        return showPlan();
      })
      .catch((error) => {
        rows = null;
        result.replaceChildren(
          h("p", { class: "note error" }, t("Could not fetch: {reason}", { reason: String(error) })),
        );
      })
      .finally(() => {
        fetchButton.disabled = false;
      });
  });

  async function showPlan(): Promise<void> {
    const view = current;
    const fetched = rows;
    if (!view || !fetched) {
      return;
    }
    const mine = ++sequence;
    const plan = await planImport(SOURCE, CURRENCY, fetched);
    if (mine === sequence) {
      result.replaceChildren(planView(view, plan));
    }
  }

  function planView(view: PortfolioView, plan: ImportPlan): HTMLElement {
    return importPlanView({
      ctx,
      view,
      plan,
      source: SOURCE,
      currency: CURRENCY,
      accountSelect,
      notes: [
        ...(unpriced.length === 0
          ? []
          : [
              h(
                "p",
                { class: "note error" },
                t(
                  usedPage
                    ? "Not on the price page, so these could not be valued and are left out: {names}"
                    : "No JPY market, so these could not be valued and are left out: {names}",
                  { names: unpriced.join(", ") },
                ),
              ),
            ]),
        ...(usedPage && compared.length > 0 ? [comparisonView()] : []),
      ],
    });
  }

  // For reference only: the page price and the Lightning last price differ in meaning and timing.
  function comparisonView(): HTMLElement {
    const yen = (amount: string) => formatAmount(amount, getLanguage());
    return h(
      "details",
      { class: "bitflyer-comparison" },
      h("summary", {}, t("Page price vs Lightning last price (for reference)")),
      h(
        "div",
        { class: "table-wrap" },
        h(
          "table",
          {},
          h(
            "thead",
            {},
            h(
              "tr",
              {},
              h("th", {}, t("Currency")),
              h("th", { class: "num" }, t("Page price")),
              h("th", { class: "num" }, t("Lightning price")),
              h("th", { class: "num" }, t("Difference")),
            ),
          ),
          h(
            "tbody",
            {},
            ...compared.map((x) =>
              h(
                "tr",
                {},
                h("td", {}, x.symbol),
                h("td", { class: "num" }, yen(x.price ?? "")),
                h("td", { class: "num" }, yen(x.api_price ?? "")),
                h("td", { class: "num" }, x.difference_percent == null ? "" : `${x.difference_percent}%`),
              ),
            ),
          ),
        ),
      ),
    );
  }

  const el = h(
    "section",
    { class: "card bitflyer" },
    h("h2", {}, "bitFlyer"),
    h(
      "p",
      { class: "note" },
      t("Create the API key with the balance-read permission only. No trading or withdrawal permission is needed."),
    ),
    credentialStatus,
    h("div", { class: "add-row" }, keyInput, secretInput, saveButton, removeKey),
    h("h3", {}, t("Balances")),
    h("div", { class: "add-row" }, accountSelect, basisSelect, fetchButton),
    pageNote,
    result,
  );

  void refreshCredentials();

  return {
    el,
    render(view) {
      current = view;
      const signature = view.portfolio.accounts.map((a) => `${a.id}:${a.name}:${a.kind}`).join(",");
      if (signature !== accountSignature) {
        accountSignature = signature;
        const previous = accountSelect.value;
        // Crypto accounts first: that is where bitFlyer balances belong.
        const ordered = [...view.portfolio.accounts].sort(
          (a, b) => Number(b.kind === "crypto") - Number(a.kind === "crypto"),
        );
        accountSelect.replaceChildren(
          ...ordered.map((a) => h("option", { value: String(a.id) }, a.name)),
        );
        if (ordered.some((a) => String(a.id) === previous)) {
          accountSelect.value = previous;
        }
      }
      if (rows) {
        void showPlan().catch((error) =>
          result.replaceChildren(h("p", { class: "note error" }, String(error))),
        );
      }
    },
  };
}
