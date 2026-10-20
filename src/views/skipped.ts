import { t } from "../i18n.ts";
import type { SkipReason, Skipped } from "../types.ts";

const SKIPPED_SHOWN = 8;

function skipLabel(reason: SkipReason): string {
  switch (reason) {
    case "missing_column":
      return t("column missing");
    case "no_name":
      return t("no name");
    case "not_a_number":
      return t("not a number");
    case "unknown_kind":
      return t("kind not in the rules");
    case "out_of_range":
      return t("negative or too large");
  }
}

/** The rows that were left out and why (the first few), for a note under a read table. */
export function skippedText(skipped: Skipped[]): string {
  const shown = skipped
    .slice(0, SKIPPED_SHOWN)
    .map((s) => `${t("row")} ${s.row} (${skipLabel(s.reason)})`)
    .join(", ");
  return t("{count} rows were left out: {rows}", {
    count: String(skipped.length),
    rows: skipped.length > SKIPPED_SHOWN ? `${shown}, …` : shown,
  });
}
