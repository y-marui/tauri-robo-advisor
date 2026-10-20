import { formatAmount } from "../decimal.ts";
import { getLanguage, t } from "../i18n.ts";
import type { ValueDifference } from "../types.ts";

/** The most rows listed; the rest are only counted. */
const DIFFERENCES_SHOWN = 8;

/** One line per row whose read value and quantity x price disagree (the first few). */
export function differenceLines(differences: ValueDifference[]): string[] {
  const language = getLanguage();
  const lines = differences.slice(0, DIFFERENCES_SHOWN).map((d) =>
    t("{name}: read {value}, quantity x price {computed}", {
      name: d.name,
      value: formatAmount(d.value, language),
      computed: formatAmount(d.computed, language),
    }),
  );
  if (differences.length > DIFFERENCES_SHOWN) {
    lines.push(`… (${differences.length - DIFFERENCES_SHOWN})`);
  }
  return lines;
}
