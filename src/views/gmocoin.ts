import {
  deleteGmocoinCredentials,
  fetchGmocoinHoldings,
  hasGmocoinCredentials,
  planImport,
  saveGmocoinCredentials,
} from "../api.ts";
import { h } from "../dom.ts";
import {
  loadExchangeAccount,
  orderAccounts,
  pickAccount,
  saveExchangeAccount,
} from "../exchange-account.ts";
import { deleteButton } from "../fields.ts";
import { t } from "../i18n.ts";
import type { Context, Panel } from "../panel.ts";
import type { ImportPlan, ImportRow, PortfolioView } from "../types.ts";
import { importPlanView } from "./import-plan.ts";

// GMO Coin: the API key, and the balances. The key is typed here, handed to
// the OS keychain by Rust, and never shown again. Balances are fetched
// read-only, linked to products through aliases the user confirms (a name is
// never guessed), and only then written over the chosen account's holdings.

const SOURCE = "gmocoin";
const CURRENCY = "JPY";

const EXCHANGE = "gmocoin";

export function gmocoinPanel(ctx: Context): Panel {
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
    deleteGmocoinCredentials()
      .then(refreshCredentials)
      .catch((error) => ctx.fail(String(error)))
      .finally(() => {
        removeKey.disabled = false;
      });
  });
  removeKey.hidden = true;

  async function refreshCredentials(): Promise<void> {
    try {
      const saved = await hasGmocoinCredentials();
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
    saveGmocoinCredentials(key, secret)
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
  const result = h("div", { class: "gmocoin-result", "aria-live": "polite" });

  let current: PortfolioView | null = null;
  let rows: ImportRow[] | null = null;
  let unpriced: string[] = [];
  // Currencies priced on the exchange board, not the sales desk: said out loud, never mixed silently.
  let boardPriced: string[] = [];
  let sequence = 0;
  let accountSignature: string | null = null;

  // The confirmation and the note name the selected account, so redraw them.
  accountSelect.addEventListener("change", () => {
    saveExchangeAccount(EXCHANGE, accountSelect.value);
    void showPlan().catch((error) =>
      result.replaceChildren(h("p", { class: "note error" }, String(error))),
    );
  });

  fetchButton.addEventListener("click", () => {
    fetchButton.disabled = true;
    result.replaceChildren(h("p", { class: "note" }, t("Fetching…")));
    fetchGmocoinHoldings()
      .then((fetched) => {
        rows = fetched.holdings.map((x) => ({
          name: x.symbol,
          value: x.value,
          quantity: x.amount,
          price: x.price ?? null,
        }));
        unpriced = fetched.unpriced;
        boardPriced = fetched.holdings.filter((x) => x.basis === "board").map((x) => x.symbol);
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
        h("p", { class: "note" }, t("Values use GMO Coin's sales desk (hanbaijo) selling price.")),
        ...(boardPriced.length === 0
          ? []
          : [
              h(
                "p",
                { class: "note error" },
                t(
                  "No sales desk price exists for these, so they use the exchange's last executed price instead: {names}",
                  { names: boardPriced.join(", ") },
                ),
              ),
            ]),
        ...(unpriced.length === 0
          ? []
          : [
              h(
                "p",
                { class: "note error" },
                t("No yen rate, so these could not be valued and are left out: {names}", {
                  names: unpriced.join(", "),
                }),
              ),
            ]),
      ],
    });
  }

  const el = h(
    "section",
    { class: "card gmocoin" },
    h("h2", {}, "GMO Coin"),
    h(
      "p",
      { class: "note" },
      t(
        "Create the API key with the permission to read the asset balance only. No trading, deposit or withdrawal permission is needed. If you can, also restrict the key to your IP address.",
      ),
    ),
    credentialStatus,
    h("div", { class: "add-row" }, keyInput, secretInput, saveButton, removeKey),
    h("h3", {}, t("Balances")),
    h("div", { class: "add-row" }, accountSelect, fetchButton),
    result,
  );

  void refreshCredentials();

  return {
    el,
    render(view) {
      current = view;
      // Accounts that hold crypto assets come first: that is where exchange balances belong.
      const ordered = orderAccounts(view);
      const signature = ordered.map((a) => `${a.id}:${a.name}`).join(",");
      if (signature !== accountSignature) {
        accountSignature = signature;
        const previous = accountSelect.value;
        accountSelect.replaceChildren(
          ...ordered.map((a) => h("option", { value: String(a.id) }, a.name)),
        );
        const picked = pickAccount(
          ordered.map((a) => String(a.id)),
          previous,
          loadExchangeAccount(EXCHANGE),
        );
        if (picked !== null) {
          accountSelect.value = picked;
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
