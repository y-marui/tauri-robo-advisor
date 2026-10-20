import { cleanDecimal } from "../decimal.ts";
import { h } from "../dom.ts";
import { deleteButton, selectField, textField } from "../fields.ts";
import { t } from "../i18n.ts";
import { panel, type Context, type Panel } from "../panel.ts";
import type { Holding, PortfolioView } from "../types.ts";
import { productTools } from "./merge-split.ts";
import { priceText } from "./holdings-table.ts";
import { recordKey, typeOptions } from "./labels.ts";

const upper = (text: string): string => text.trim().toUpperCase();

/** The number of columns of the records table; the heading rows span them all. */
const COLUMNS = 7;

/** What to focus once the card has been rebuilt: the rebuild drops the focus of the old controls. */
type FocusTarget = { kind: "add-record"; product: number } | { kind: "add-product" };

/**
 * Every product and every record, as saved: the one place where they are edited.
 * One table; each product has a heading row (name, category, currency, delete,
 * add a record) and one row per record (account x product).
 */
export function recordsPanel(ctx: Context): Panel {
  let current: PortfolioView | null = null;
  let blocks: { product: number; el: HTMLElement; category: HTMLElement }[] = [];
  let pendingFocus: FocusTarget | null = null;

  // The filter sits outside the part that is rebuilt, so rebuilding does not clear it.
  const filter = h("input", {
    type: "search",
    "aria-label": t("Filter products"),
    placeholder: t("Filter products"),
    autocomplete: "off",
    class: "records-filter",
  });
  const applyFilter = (): void => {
    const needle = filter.value.trim().toLowerCase();
    const shown = new Set<HTMLElement>();
    for (const block of blocks) {
      const name = current?.portfolio.products.find((p) => p.id === block.product)?.name ?? "";
      const visible = needle === "" || name.toLowerCase().includes(needle);
      block.el.hidden = !visible;
      if (visible) {
        shown.add(block.category);
      }
    }
    for (const category of new Set(blocks.map((b) => b.category))) {
      category.hidden = !shown.has(category);
    }
  };
  filter.addEventListener("input", applyFilter);

  const card = panel(
    "Records",
    (view) =>
      [
        view.portfolio.categories.map((c) => `${c.id}:${c.name}:${c.targeting}`).join(","),
        view.portfolio.products
          .map((p) => `${p.id}:${p.category}:${p.currency}:${p.aliases.map((a) => `${a.source}/${a.name}`).join(";")}`)
          .join(","),
        view.portfolio.accounts.map((a) => `${a.id}:${a.name}`).join(","),
        view.portfolio.holdings.map((r) => recordKey(r.account, r.product, r.account_type)).join(","),
        (view.portfolio.custom_types ?? []).map((c) => `${c.id}:${c.name}`).join(","),
      ].join("|"),
    (view, bind) => {
      const { categories, accounts, products } = view.portfolio;
      blocks = [];
      if (categories.length === 0) {
        return [h("p", { class: "note" }, t("Add a category first."))];
      }
      bind((v) => {
        current = v;
        applyFilter();
      });
      const categoryOptions = categories.map((c) => ({ value: String(c.id), label: c.name }));
      const focusers = new Map<string, HTMLElement>();

      const typeChoices = typeOptions(view.portfolio.custom_types);
      const holdingOf = (
        v: PortfolioView,
        account: number,
        product: number,
        type: string | null,
      ): Holding | undefined =>
        v.portfolio.holdings.find(
          (r) => r.account === account && r.product === product && (r.account_type ?? null) === type,
        );

      const recordRow = (
        product: (typeof products)[number],
        account: (typeof accounts)[number],
        type: string | null,
        weighted: boolean,
      ): HTMLElement => {
        const quantity = h("td", { class: "num" });
        const price = h("td", { class: "num" });
        bind((v) => {
          const held = holdingOf(v, account.id, product.id, type);
          quantity.textContent = held?.quantity ?? "";
          price.textContent = priceText(held);
        });
        const amount = textField({
          ctx,
          bind,
          label: `${product.name} / ${account.name}: ${t("Amount")}`,
          get: (v) => holdingOf(v, account.id, product.id, type)?.value ?? "",
          edit: (text) => {
            const value = cleanDecimal(text);
            if (value === null) {
              // An empty amount does not delete the record; put the saved amount back.
              if (text === "" && current) {
                amount.value = holdingOf(current, account.id, product.id, type)?.value ?? "";
              }
              return null;
            }
            return {
              type: "set_holding",
              account: account.id,
              product: product.id,
              account_type: type,
              value,
              quantity: current ? (holdingOf(current, account.id, product.id, type)?.quantity ?? null) : null,
            };
          },
          invalid: t("Enter an amount (0 is allowed). To remove a record, use its Delete button."),
          attrs: { inputmode: "decimal", class: "number" },
        });
        const weight = weighted
          ? textField({
              ctx,
              bind,
              label: t("Relative weight inside the category: {account}", { account: account.name }),
              get: (v) => holdingOf(v, account.id, product.id, type)?.weight ?? "",
              edit: (text) => {
                const value = text === "" ? null : cleanDecimal(text);
                return text !== "" && value === null
                  ? null
                  : {
                      type: "set_holding_weight",
                      account: account.id,
                      product: product.id,
                      account_type: type,
                      weight: value,
                    };
              },
              attrs: { inputmode: "decimal", class: "number" },
            })
          : null;
        return h(
          "tr",
          {},
          h("td", { class: "clip", title: account.name }, account.name),
          h(
            "td",
            {},
            selectField({
              ctx,
              bind,
              label: `${product.name} / ${account.name}: ${t("Type")}`,
              options: typeChoices,
              get: () => type ?? "",
              edit: (value) => ({
                type: "set_record_type",
                account: account.id,
                product: product.id,
                from: type,
                to: value === "" ? null : value,
              }),
            }),
          ),
          h("td", {}, weight),
          quantity,
          price,
          h("td", {}, amount, " ", h("span", { class: "muted" }, product.currency)),
          h(
            "td",
            {},
            deleteButton(
              () =>
                void ctx.apply({
                  type: "remove_holding",
                  account: account.id,
                  product: product.id,
                  account_type: type,
                }),
            ),
          ),
        );
      };

      const addRecordForm = (
        product: (typeof products)[number],
        weighted: boolean,
      ): HTMLElement | null => {
        if (accounts.length === 0) {
          return null;
        }
        const account = h(
          "select",
          { "aria-label": `${product.name}: ${t("Account")}` },
          ...accounts.map((a) => h("option", { value: String(a.id) }, a.name)),
        );
        const type = h(
          "select",
          { "aria-label": `${product.name}: ${t("Type")}` },
          ...typeChoices.map((o) => h("option", { value: o.value }, o.label)),
        );
        const amount = h("input", {
          type: "text",
          "aria-label": `${product.name}: ${t("Amount")}`,
          placeholder: t("Amount"),
          inputmode: "decimal",
          autocomplete: "off",
          class: "number",
        });
        const weight = weighted
          ? h("input", {
              type: "text",
              "aria-label": `${product.name}: ${t("Weight")}`,
              placeholder: t("Weight"),
              inputmode: "decimal",
              autocomplete: "off",
              class: "number",
            })
          : null;
        const form = h(
          "form",
          { class: "record-add" },
          account,
          type,
          amount,
          weight,
          h("button", { type: "submit" }, t("Add a record")),
        );
        focusers.set(`record:${product.id}`, account);
        form.addEventListener("submit", (event) => {
          event.preventDefault();
          const accountId = Number(account.value);
          const accountType = type.value === "" ? null : type.value;
          const amountText = amount.value.trim();
          const weightText = weight?.value.trim() ?? "";
          const value = amountText === "" ? null : cleanDecimal(amountText);
          const share = weightText === "" ? null : cleanDecimal(weightText);
          if ((amountText === "" && weightText === "") || (amountText !== "" && value === null) || (weightText !== "" && share === null)) {
            ctx.fail(t("Enter an amount or a weight, as numbers."));
            return;
          }
          pendingFocus = { kind: "add-record", product: product.id };
          void (async () => {
            let ok = true;
            if (value !== null) {
              ok = await ctx.apply({
                type: "set_holding",
                account: accountId,
                product: product.id,
                account_type: accountType,
                value,
                quantity: null,
              });
            }
            if (ok && share !== null) {
              ok = await ctx.apply({
                type: "set_holding_weight",
                account: accountId,
                product: product.id,
                account_type: accountType,
                weight: share,
              });
            }
            if (!ok) {
              pendingFocus = null;
            }
          })();
        });
        return form;
      };

      const bodies: HTMLElement[] = [];
      for (const category of categories) {
        const weighted = category.targeting === "weighted";
        const heading = h(
          "tbody",
          { class: "records-category" },
          h("tr", {}, h("th", { colspan: COLUMNS, scope: "colgroup" }, category.name)),
        );
        bodies.push(heading);
        for (const product of products.filter((p) => p.category === category.id)) {
          const id = product.id;
          const find = (v: PortfolioView) => v.portfolio.products.find((p) => p.id === id);
          // The name box grows with the name (the full name is also the tooltip), so two products
          // with similar names can be told apart.
          const nameField = (): HTMLInputElement => {
            const input = textField({
              ctx,
              bind,
              label: t("Product name"),
              get: (v) => find(v)?.name ?? "",
              edit: (text) => ({ type: "update_product", id, name: text }),
              attrs: { class: "product-name" },
            });
            const fit = (): void => {
              input.title = input.value;
              input.style.width = `max(12rem, calc(${[...input.value].length}em + 2rem))`;
            };
            bind(fit);
            input.addEventListener("input", fit);
            return input;
          };
          const head = h(
            "div",
            { class: "record-head" },
            nameField(),
            selectField({
              ctx,
              bind,
              label: t("Category"),
              options: categoryOptions,
              get: (v) => String(find(v)?.category ?? ""),
              edit: (value) => ({ type: "update_product", id, category: Number(value) }),
            }),
            textField({
              ctx,
              bind,
              label: t("Currency"),
              get: (v) => find(v)?.currency ?? "",
              normalize: upper,
              invalid: t("Currency must be three letters, for example JPY."),
              edit: (text) =>
                /^[A-Z]{3}$/.test(text) ? { type: "update_product", id, currency: text } : null,
              attrs: { class: "currency", maxlength: "3" },
            }),
            deleteButton(() => void ctx.apply({ type: "remove_product", id }), t("Delete product")),
            addRecordForm(product, weighted),
            productTools(ctx, view, product),
          );
          const own = view.portfolio.holdings.filter((r) => r.product === id);
          const rows = accounts.flatMap((a) =>
            own
              .filter((r) => r.account === a.id)
              .map((r) => recordRow(product, a, r.account_type ?? null, weighted)),
          );
          const block = h(
            "tbody",
            { class: "record-product" },
            h("tr", {}, h("td", { colspan: COLUMNS, class: "record-head-cell" }, head)),
            ...(rows.length > 0
              ? rows
              : [h("tr", {}, h("td", { colspan: COLUMNS, class: "muted" }, t("No records")))]),
          );
          bodies.push(block);
          blocks.push({ product: id, el: block, category: heading });
        }
      }

      const name = h("input", { type: "text", "aria-label": t("Name"), autocomplete: "off" });
      const category = h(
        "select",
        { "aria-label": t("Category") },
        ...categoryOptions.map((o) => h("option", { value: o.value }, o.label)),
      );
      const currency = h("input", {
        type: "text",
        "aria-label": t("Currency"),
        class: "currency",
        maxlength: "3",
        value: "JPY",
      });
      const form = h(
        "form",
        { class: "add-row" },
        name,
        category,
        currency,
        h("button", { type: "submit" }, t("Add a product")),
      );
      focusers.set("product", name);
      form.addEventListener("submit", (event) => {
        event.preventDefault();
        pendingFocus = { kind: "add-product" };
        void ctx
          .apply({
            type: "add_product",
            name: name.value,
            category: Number(category.value),
            currency: upper(currency.value),
          })
          .then((ok) => {
            if (!ok) {
              pendingFocus = null;
            }
          });
      });

      // The new controls are attached after this returns; put the focus back then.
      const target = pendingFocus;
      pendingFocus = null;
      if (target) {
        queueMicrotask(() =>
          focusers.get(target.kind === "add-record" ? `record:${target.product}` : "product")?.focus(),
        );
      }

      return [
        h(
          "div",
          { class: "table-wrap" },
          h(
            "table",
            { class: "records" },
            h(
              "thead",
              {},
              h(
                "tr",
                {},
                h("th", {}, t("Account")),
                h("th", {}, t("Type")),
                h("th", {}, t("Weight")),
                h("th", {}, t("Quantity")),
                h("th", {}, t("Price")),
                h("th", {}, t("Amount")),
                h("th", {}),
              ),
            ),
            ...bodies,
          ),
        ),
        form,
      ];
    },
  );

  const tools = h(
    "div",
    { class: "records-tools" },
    h(
      "p",
      { class: "note" },
      t("Every product and every record, as saved. Edit here; the Holdings tab only shows them."),
      " ",
      t("Amounts are in each product’s currency."),
      " ",
      t("Category and group names are edited in the Categories and Groups cards."),
    ),
    filter,
  );
  card.el.insertBefore(tools, card.el.querySelector(".card-body"));
  return {
    ...card,
    reveal() {
      card.el.scrollIntoView({ block: "start" });
      filter.focus({ preventScroll: true });
    },
  };
}
