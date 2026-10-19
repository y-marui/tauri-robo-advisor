import { t } from "../i18n.ts";
import type { AccountKind, ProductTargeting } from "../types.ts";

export const TARGETINGS: ProductTargeting[] = ["free", "equal", "keep_current", "weighted"];
export const KINDS: AccountKind[] = [
  "taxable",
  "nisa",
  "ideco",
  "points",
  "crypto",
  "deposit",
  "other",
];

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

export function kindLabel(value: AccountKind): string {
  switch (value) {
    case "taxable":
      return t("Taxable account");
    case "nisa":
      return t("NISA");
    case "ideco":
      return t("iDeCo");
    case "points":
      return t("Points");
    case "crypto":
      return t("Crypto exchange");
    case "deposit":
      return t("Deposit");
    case "other":
      return t("Other");
  }
}
