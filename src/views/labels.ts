import { t } from "../i18n.ts";
import type { CustomType, ProductTargeting } from "../types.ts";

export const TARGETINGS: ProductTargeting[] = ["free", "equal", "keep_current", "weighted"];
/** The account types that ship with the app, in the order they are offered. */
export const BUILTIN_TYPES = [
  "general",
  "specific",
  "nisa",
  "nisa_growth",
  "nisa_tsumitate",
  "old_nisa",
  "deposit",
  "crypto",
] as const;

export function targetingLabel(value: ProductTargeting): string {
  switch (value) {
    case "free":
      return t("Free");
    case "equal":
      return t("Equal");
    case "keep_current":
      return t("Keep current");
    case "weighted":
      return t("Weighted");
  }
}

export function typeLabel(id: string | null | undefined, custom: CustomType[] | undefined): string {
  switch (id) {
    case undefined:
    case null:
      return t("Not set");
    case "general":
      return t("General account");
    case "specific":
      return t("Specific account");
    case "nisa":
      return t("NISA (unspecified)");
    case "nisa_growth":
      return t("NISA growth");
    case "nisa_tsumitate":
      return t("NISA tsumitate");
    case "old_nisa":
      return t("Old NISA");
    case "deposit":
      return t("Cash");
    case "crypto":
      return t("Crypto assets");
    default:
      return custom?.find((c) => c.id === id)?.name ?? id;
  }
}

/** The choices of a type: not set, the built-in types, then the user's. */
export function typeOptions(custom: CustomType[] | undefined): { value: string; label: string }[] {
  return [
    { value: "", label: typeLabel(null, custom) },
    ...BUILTIN_TYPES.map((id) => ({ value: id, label: typeLabel(id, custom) })),
    ...(custom ?? []).map((c) => ({ value: c.id, label: c.name })),
  ];
}

/** Identifies a record in the screen (account, product, type). */
export function recordKey(account: number, product: number, type: string | null | undefined): string {
  return `${account}:${product}:${type ?? ""}`;
}
