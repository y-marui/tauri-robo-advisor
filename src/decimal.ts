// Exact decimal text helpers.
//
// The frontend never does arithmetic on money or weights: floating point would
// round them. These helpers only move the decimal point of a string, so a
// percentage typed by the user and the fraction Rust stores stay exact.

const DECIMAL = /^(-)?(\d*)(?:\.(\d*))?$/;

/** Trims leading and trailing zeros: "007.500" -> "7.5", ".5" -> "0.5". */
function normalize(sign: string, whole: string, fraction: string): string {
  const w = whole.replace(/^0+/, "") || "0";
  const f = fraction.replace(/0+$/, "");
  const body = f ? `${w}.${f}` : w;
  return body === "0" ? "0" : `${sign}${body}`;
}

/**
 * Moves the decimal point `places` digits to the right (negative: left).
 * Returns `null` when the text is not a plain decimal number.
 */
export function shiftDecimal(text: string, places: number): string | null {
  const match = DECIMAL.exec(text.trim());
  if (!match || (match[2] === "" && (match[3] ?? "") === "")) {
    return null;
  }
  const sign = match[1] ?? "";
  let whole = match[2] ?? "";
  let fraction = match[3] ?? "";
  if (places >= 0) {
    fraction = fraction.padEnd(places, "0");
    whole += fraction.slice(0, places);
    fraction = fraction.slice(places);
  } else {
    const shift = -places;
    whole = whole.padStart(shift, "0");
    fraction = whole.slice(whole.length - shift) + fraction;
    whole = whole.slice(0, whole.length - shift);
  }
  return normalize(sign, whole, fraction);
}

/** "30" (percent) -> "0.3" (fraction); `null` for text that is not a number. */
export function percentToFraction(percent: string): string | null {
  return shiftDecimal(percent, -2);
}

/** "0.3" (fraction) -> "30" (percent). */
export function fractionToPercent(fraction: string): string {
  return shiftDecimal(fraction, 2) ?? fraction;
}

/** A plain non-negative or signed decimal, or `null`; used to validate input. */
export function cleanDecimal(text: string): string | null {
  return shiftDecimal(text, 0);
}

/**
 * Formats an exact decimal string for display with the locale's digit
 * grouping, without converting it to a floating-point number first.
 */
export function formatAmount(amount: string, locale: string): string {
  const clean = cleanDecimal(amount);
  if (clean === null) {
    return amount;
  }
  // Rust gives an amount with the digits it should show (such as "10.50"), so the digits of the
  // string are kept, trailing zeros included.
  // Capped at 20, the limit older engines accept for the digit options.
  const digits = Math.min(amount.trim().split(".")[1]?.length ?? 0, 20);
  return new Intl.NumberFormat(locale, {
    minimumFractionDigits: digits,
    maximumFractionDigits: 20,
  }).format(
    clean as unknown as number,
  );
}
