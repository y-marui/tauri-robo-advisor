import { fractionToPercent } from "../decimal.ts";
import { h } from "../dom.ts";
import { t } from "../i18n.ts";
import type { Issue } from "../types.ts";

/** The text of one issue; category names are the user's own and not translated. */
export function describeIssue(issue: Issue): string {
  switch (issue.type) {
    case "category_weights_do_not_sum_to_one":
      return t("Category targets add up to {sum}%, not 100%.", {
        sum: fractionToPercent(issue.sum),
      });
    case "missing_exchange_rate":
      return t("Set the {currency}/JPY rate in Settings to value {currency} holdings.", {
        currency: issue.currency,
      });
  }
}

/** A list of the problems that block suggestions; empty when there are none. */
export function issuesList(issues: Issue[]): HTMLElement | null {
  if (issues.length === 0) {
    return null;
  }
  return h(
    "ul",
    { class: "issues", role: "status" },
    ...issues.map((issue) => h("li", {}, describeIssue(issue))),
  );
}
