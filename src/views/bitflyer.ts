import {
  deleteBitflyerCredentials,
  fetchBitflyerHoldings,
  hasBitflyerCredentials,
  planImport,
  saveBitflyerCredentials,
} from "../api.ts";
import { formatAmount } from "../decimal.ts";
import { h } from "../dom.ts";
import { deleteButton } from "../fields.ts";
import { getLanguage, t } from "../i18n.ts";
import type { Context, Panel } from "../panel.ts";
import type { ImportPlan, ImportRow, PortfolioView } from "../types.ts";

// bitFlyer: the API key, and the balances. The key is typed here, handed to
// the OS keychain by Rust, and never shown again. Balances are fetched
// read-only, linked to products through aliases the user confirms (a name is
// never guessed), and only then written over the chosen account's holdings.

const SOURCE = "bitflyer";
const CURRENCY = "JPY";

const yen = (amount: string): string => formatAmount(amount, getLanguage());

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
  const result = h("div", { class: "bitflyer-result", "aria-live": "polite" });

  let current: PortfolioView | null = null;
  let rows: ImportRow[] | null = null;
  let unpriced: string[] = [];
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
    fetchBitflyerHoldings()
      .then((fetched) => {
        rows = fetched.holdings.map((x) => ({ name: x.symbol, value: x.value }));
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
    const productName = (id: number) => view.portfolio.products.find((p) => p.id === id)?.name ?? "?";
    const parts: (HTMLElement | null)[] = [];

    if (plan.matched.length > 0) {
      parts.push(
        h(
          "div",
          { class: "table-wrap" },
          h(
            "table",
            {},
            h(
              "thead",
              {},
              h("tr", {}, h("th", {}, t("Currency")), h("th", {}, t("Product")), h("th", {}, t("Value (JPY)"))),
            ),
            h(
              "tbody",
              {},
              ...plan.matched.map((row) =>
                h(
                  "tr",
                  {},
                  h("th", { scope: "row" }, row.name),
                  h("td", {}, productName(row.product)),
                  h("td", { class: "num" }, yen(row.value)),
                ),
              ),
            ),
          ),
        ),
      );
    }

    if (plan.unmatched.length > 0) {
      const options = view.portfolio.products
        .filter((p) => p.currency === CURRENCY)
        .map((p) => h("option", { value: String(p.id) }, p.name));
      parts.push(
        h("p", { class: "note" }, t("Link each name to a product. The choice is remembered.")),
        ...plan.unmatched.map((row) => {
          const select = h("select", { "aria-label": `${row.name} → ${t("Product")}` }, ...options);
          const link = h("button", { type: "button" }, t("Link"));
          link.disabled = options.length === 0;
          link.addEventListener("click", () => {
            void ctx.apply({
              type: "add_alias",
              product: Number(select.value),
              source: SOURCE,
              name: row.name,
            });
          });
          return h(
            "div",
            { class: "add-row" },
            h("span", {}, `${row.name} (${yen(row.value)} JPY)`),
            select,
            link,
          );
        }),
        options.length === 0 ? h("p", { class: "note" }, t("Add a JPY product first.")) : null,
      );
    }

    if (plan.wrong_currency.length > 0) {
      parts.push(
        h(
          "p",
          { class: "note error" },
          t("Linked to a product that is not in JPY, so they are left out: {names}", {
            names: plan.wrong_currency.map((r) => r.name).join(", "),
          }),
        ),
      );
    }
    if (unpriced.length > 0) {
      parts.push(
        h(
          "p",
          { class: "note error" },
          t("No JPY market, so these could not be valued and are left out: {names}", {
            names: unpriced.join(", "),
          }),
        ),
      );
    }

    const apply = h("button", { type: "button" }, t("Replace holdings"));
    apply.disabled = accountSelect.value === "";
    let armed: number | undefined;
    apply.addEventListener("click", () => {
      if (armed === undefined) {
        apply.textContent = t("Confirm replace");
        apply.classList.add("danger", "armed");
        armed = window.setTimeout(() => {
          apply.textContent = t("Replace holdings");
          apply.classList.remove("danger", "armed");
          armed = undefined;
        }, 4000);
        return;
      }
      window.clearTimeout(armed);
      armed = undefined;
      // Read the account now, not when the plan was drawn: the user may have
      // changed the selection in between.
      const accountId = Number(accountSelect.value);
      if (accountSelect.value === "" || !Number.isFinite(accountId)) {
        ctx.fail(t("Choose an account first."));
        return;
      }
      void ctx.apply({
        type: "replace_holdings",
        account: accountId,
        holdings: plan.matched.map((m) => ({ product: m.product, value: m.value })),
      });
    });
    parts.push(
      h(
        "p",
        { class: "note" },
        t("This replaces every holding of “{account}” with the linked balances above.", {
          account:
            view.portfolio.accounts.find((a) => String(a.id) === accountSelect.value)?.name ?? "?",
        }),
      ),
      apply,
    );
    return h("div", {}, ...parts);
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
    h("div", { class: "add-row" }, accountSelect, fetchButton),
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
