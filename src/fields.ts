import { cleanDecimal, fractionToPercent, percentToFraction } from "./decimal.ts";
import { h } from "./dom.ts";
import { t } from "./i18n.ts";
import type { Bind, Context } from "./panel.ts";
import type { Edit, PortfolioView } from "./types.ts";

type Binder = (binding: Bind) => void;

function syncValue(input: HTMLInputElement | HTMLSelectElement, value: string): void {
  if (document.activeElement !== input && input.value !== value) {
    input.value = value;
  }
}

interface TextOptions {
  ctx: Context;
  bind: Binder;
  label: string;
  /** The current value in the model. */
  get: (view: PortfolioView) => string;
  /** The edit for a new value, or `null` when the text is not acceptable. */
  edit: (text: string) => Edit | null;
  /** Shown when `edit` returns `null`. */
  invalid?: string;
  attrs?: Record<string, string>;
  /** Normalises typed text before it is shown or sent. */
  normalize?: (text: string) => string;
}

export function textField(options: TextOptions): HTMLInputElement {
  const { ctx, bind, label, get, edit, attrs } = options;
  const input = h("input", { type: "text", "aria-label": label, autocomplete: "off", ...attrs });
  input.addEventListener("change", () => {
    const text = options.normalize ? options.normalize(input.value) : input.value.trim();
    input.value = text;
    const next = edit(text);
    if (next === null) {
      ctx.fail(options.invalid ?? t("Enter a valid number."));
      return;
    }
    void ctx.apply(next);
  });
  bind((view) => syncValue(input, get(view)));
  return input;
}

/** A percentage the user types ("30"); Rust receives the exact fraction ("0.3"). */
export function percentField(
  options: Omit<TextOptions, "edit" | "attrs"> & { save: (fraction: string) => Edit },
): HTMLInputElement {
  return textField({
    ...options,
    get: (view) => fractionToPercent(options.get(view)),
    edit: (text) => {
      const fraction = percentToFraction(text);
      return fraction === null ? null : options.save(fraction);
    },
    attrs: { inputmode: "decimal", class: "number" },
  });
}

/** An amount or plain number, sent as the cleaned decimal text. */
export function amountField(
  options: Omit<TextOptions, "edit" | "attrs"> & { save: (value: string) => Edit },
): HTMLInputElement {
  return textField({
    ...options,
    edit: (text) => {
      const clean = cleanDecimal(text);
      return clean === null ? null : options.save(clean);
    },
    attrs: { inputmode: "decimal", class: "number" },
  });
}

interface SelectOptions {
  ctx: Context;
  bind: Binder;
  label: string;
  options: { value: string; label: string }[];
  get: (view: PortfolioView) => string;
  edit: (value: string) => Edit;
}

export function selectField(opts: SelectOptions): HTMLSelectElement {
  const select = h(
    "select",
    { "aria-label": opts.label },
    ...opts.options.map((o) => h("option", { value: o.value }, o.label)),
  );
  select.addEventListener("change", () => void opts.ctx.apply(opts.edit(select.value)));
  opts.bind((view) => syncValue(select, opts.get(view)));
  return select;
}

interface CheckboxOptions {
  ctx: Context;
  bind: Binder;
  label: string;
  get: (view: PortfolioView) => boolean;
  edit: (checked: boolean) => Edit;
}

export function checkboxField(opts: CheckboxOptions): HTMLInputElement {
  const box = h("input", { type: "checkbox", "aria-label": opts.label });
  box.addEventListener("change", () => void opts.ctx.apply(opts.edit(box.checked)));
  opts.bind((view) => {
    box.checked = opts.get(view);
  });
  return box;
}

/**
 * A delete button that asks twice: the first click arms it, the second within
 * a few seconds deletes. (`window.confirm` is not reliable in the Tauri webview.)
 */
export function deleteButton(
  onConfirm: () => void,
  label: string = t("Delete"),
): HTMLButtonElement {
  const button = h("button", { type: "button", class: "danger" }, label);
  let timer: number | undefined;
  button.addEventListener("click", () => {
    if (timer === undefined) {
      button.textContent = t("Confirm delete");
      button.classList.add("armed");
      timer = window.setTimeout(() => {
        button.textContent = label;
        button.classList.remove("armed");
        timer = undefined;
      }, 4000);
      return;
    }
    window.clearTimeout(timer);
    timer = undefined;
    onConfirm();
  });
  return button;
}
