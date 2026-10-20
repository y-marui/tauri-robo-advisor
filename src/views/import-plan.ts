import { formatAmount } from "../decimal.ts";
import { h } from "../dom.ts";
import { getLanguage, t } from "../i18n.ts";
import type { Context } from "../panel.ts";
import type { Edit, ImportPlan, PortfolioView } from "../types.ts";

// The result of an import, shared by every source (bitFlyer, a table file or
// pasted text): the rows linked to products, a way to link the others by hand,
// and the confirmed replacement of an account's holdings. Names are never
// linked automatically: the user picks the product and the choice is remembered
// as an alias of this source.

const yen = (amount: string): string => formatAmount(amount, getLanguage());

export interface ImportPlanOptions {
  ctx: Context;
  view: PortfolioView;
  plan: ImportPlan;
  /** The alias source, e.g. "bitflyer" or the name the user gave a table. */
  source: string;
  /** The currency of the values; only products in this currency can be linked. */
  currency: string;
  /** The account to replace; without it the view only shows and links, and has no replace button. */
  accountSelect?: HTMLSelectElement;
  /** Extra messages shown above the replace button. */
  notes?: HTMLElement[];
}

/** The linked rows, the rest to link, and the button that replaces the holdings. */
export function importPlanView(options: ImportPlanOptions): HTMLElement {
  const { view, plan, source, currency, accountSelect, notes } = options;
  // Every edit is tied to the data location of the view the plan was made from, so a plan that
  // resolves after the data folder was switched cannot change the new data.
  const ctx = {
    ...options.ctx,
    apply: (edit: Edit) => options.ctx.apply(edit, view.data.location),
  };
  const productName = (id: number) => view.portfolio.products.find((p) => p.id === id)?.name ?? "?";
  const parts: (HTMLElement | null)[] = [];

  // A wrong link is undone here: removing the alias puts the name back among
  // the unmatched ones, where it can be linked again.
  const unlink = (productId: number, name: string): HTMLButtonElement => {
    const button = h("button", { type: "button" }, t("Unlink"));
    button.setAttribute("aria-label", `${t("Unlink")} ${name}`);
    button.addEventListener("click", () => {
      void ctx.apply({ type: "remove_alias", product: productId, source, name });
    });
    return button;
  };

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
            h(
              "tr",
              {},
              h("th", {}, t("Name")),
              h("th", {}, t("Product")),
              h("th", {}, currency === "" ? t("Value") : t("Value ({currency})", { currency })),
              h("th", {}, ""),
            ),
          ),
          h(
            "tbody",
            {},
            ...plan.matched.map((row) =>
              h(
                "tr",
                {},
                h("th", { scope: "row" }, row.names.join(", ")),
                h("td", {}, productName(row.product)),
                h("td", { class: "num" }, yen(row.value)),
                h("td", {}, ...row.names.map((name) => unlink(row.product, name))),
              ),
            ),
          ),
        ),
      ),
    );
  }

  if (plan.unmatched.length > 0) {
    // An empty currency means "the product's own": any product can be linked, and its own
    // currency is shown next to its name.
    const candidates = view.portfolio.products.filter((p) => currency === "" || p.currency === currency);
    // Every row needs its own option elements: appending one element to a second
    // select would move it out of the first.
    const optionsFor = (): HTMLOptionElement[] =>
      candidates.map((p) => h("option", { value: String(p.id) }, currency === "" ? `${p.name} (${p.currency})` : p.name));
    // A name that has no product yet can become one here: the name, a category, and a currency
    // (the one the table was imported in, or another), then it is linked in the same step.
    const categories = view.portfolio.categories;
    const currencies = new Set<string>(["JPY", "USD"]);
    view.portfolio.products.forEach((p) => currencies.add(p.currency));
    const defaultCurrency = currency === "" ? "JPY" : currency;
    currencies.add(defaultCurrency);
    parts.push(
      h("p", { class: "note" }, t("Link each name to a product. The choice is remembered.")),
      ...plan.unmatched.map((row) => {
        const select = h("select", { "aria-label": `${row.name} → ${t("Product")}` }, ...optionsFor());
        const link = h("button", { type: "button" }, t("Link"));
        link.disabled = candidates.length === 0;
        link.addEventListener("click", () => {
          void ctx.apply({
            type: "add_alias",
            product: Number(select.value),
            source: source,
            name: row.name,
          });
        });
        const productName = h("input", { type: "text", autocomplete: "off", value: row.name, "aria-label": t("Name of the new product") });
        const categorySelect = h(
          "select",
          { "aria-label": t("Category of the new product") },
          ...categories.map((c) => h("option", { value: String(c.id) }, c.name)),
        );
        // Any currency code can be typed (the list offers the usual ones); Rust checks the code.
        const currencyList = h("datalist", {}, ...[...currencies].sort().map((c) => h("option", { value: c })));
        const listId = `currencies-${Math.random().toString(36).slice(2)}`;
        currencyList.id = listId;
        const currencySelect = h("input", {
          type: "text",
          autocomplete: "off",
          class: "currency",
          list: listId,
          value: defaultCurrency,
          "aria-label": t("Currency of the new product"),
        });
        const create = h("button", { type: "button" }, t("Create a product and link"));
        create.disabled = categories.length === 0;
        create.addEventListener("click", () => {
          void ctx.apply({
            type: "add_product_with_alias",
            name: productName.value.trim(),
            category: Number(categorySelect.value),
            currency: currencySelect.value.trim().toUpperCase(),
            source: source,
            alias: row.name,
          });
        });
        return h(
          "div",
          { class: "unmatched" },
          h(
            "div",
            { class: "add-row" },
            h("span", {}, `${row.name} (${yen(row.value)}${currency === "" ? "" : ` ${currency}`})`),
            select,
            link,
          ),
          h("div", { class: "add-row" }, productName, categorySelect, currencySelect, currencyList, create),
        );
      }),
      candidates.length === 0
        ? h("p", { class: "note" }, currency === "" ? t("Add a product first.") : t("Add a {currency} product first.", { currency }))
        : null,
      categories.length === 0
        ? h("p", { class: "note" }, t("To make a product from a name, add a category first (in the Categories card)."))
        : null,
    );
  }

  if (plan.wrong_currency.length > 0) {
    parts.push(
      h(
        "p",
        { class: "note error" },
        t("Linked to a product that is not in {currency}, so they are left out: {names}", {
          currency,
          names: plan.wrong_currency.map((r) => r.name).join(", "),
        }),
      ),
      ...plan.wrong_currency.map((row) => {
        const owner = view.portfolio.products.find((p) =>
          p.aliases.some((a) => a.source === source && a.name === row.name),
        );
        return owner ? h("div", { class: "add-row" }, h("span", {}, row.name), unlink(owner.id, row.name)) : null;
      }),
    );
  }
  parts.push(...(notes ?? []));

  if (accountSelect) {
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
        holdings: plan.matched.map((m) => ({
          product: m.product,
          value: m.value,
          quantity: m.quantity,
          price: m.price,
          price_unit: m.price_unit,
        })),
      });
    });
    parts.push(
      h(
        "p",
        { class: "note" },
        t("This replaces every holding of “{account}” with the linked rows above.", {
          account:
            view.portfolio.accounts.find((a) => String(a.id) === accountSelect.value)?.name ?? "?",
        }),
      ),
      apply,
    );
  }
  return h("div", {}, ...parts);
}
