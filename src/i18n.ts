// UI text. English is the development language and the fallback: the English
// text is the key. Other languages are translated from it (see
// docs/dev-charter/LOCALIZATION_POLICY.md). User data such as category and
// product names is never translated.

export type Lang = "en" | "ja";

export const LANGUAGES: { code: Lang; label: string }[] = [
  { code: "en", label: "English" },
  { code: "ja", label: "日本語" },
];

export const MESSAGES = {
  "Portfolio": { ja: "ポートフォリオ" },
  "Language": { ja: "言語" },
  "Categories": { ja: "カテゴリ" },
  "Products": { ja: "商品" },
  "Accounts": { ja: "口座" },
  "Holdings": { ja: "保有" },
  "Settings": { ja: "設定" },
  "Name": { ja: "名前" },
  "Target (%)": { ja: "目標 (%)" },
  "Product targeting": { ja: "商品の目標の持ち方" },
  "Category": { ja: "カテゴリ" },
  "Currency": { ja: "通貨" },
  "Weight": { ja: "比率" },
  "Type": { ja: "種別" },
  "Sellable": { ja: "売却可" },
  "Funding": { ja: "原資" },
  "Rounding unit": { ja: "丸め単位" },
  "Can buy": { ja: "買える商品" },
  "Any product": { ja: "すべての商品" },
  "Add": { ja: "追加" },
  "Delete": { ja: "削除" },
  "Confirm delete": { ja: "削除する" },
  "Add a category": { ja: "カテゴリを追加" },
  "Add a product": { ja: "商品を追加" },
  "Add an account": { ja: "口座を追加" },
  "Add a category first.": { ja: "先にカテゴリを追加してください。" },
  "Add products and accounts to enter holdings.": {
    ja: "保有を入力するには、商品と口座を追加してください。",
  },
  "Free": { ja: "自由" },
  "Equal": { ja: "等分" },
  "Keep current": { ja: "現在の比率を維持" },
  "Weighted": { ja: "比率を指定" },
  "Taxable account": { ja: "特定口座" },
  "NISA": { ja: "NISA" },
  "iDeCo": { ja: "iDeCo" },
  "Points": { ja: "ポイント" },
  "Crypto exchange": { ja: "暗号資産" },
  "Deposit": { ja: "預金" },
  "Other": { ja: "その他" },
  "Cash": { ja: "現金" },
  "USD/JPY rate": { ja: "USD/JPY レート" },
  "Category tolerance (%)": { ja: "カテゴリの許容乖離 (%)" },
  "Product tolerance (%)": { ja: "商品の許容乖離 (%)" },
  "Rebalancing account": { ja: "全体リバランスの対象口座" },
  "None": { ja: "なし" },
  "Value": { ja: "評価額" },
  "Export": { ja: "エクスポート" },
  "Exporting…": { ja: "書き出し中…" },
  "Exported to {folder}. The files contain your holdings; keep them private.": {
    ja: "{folder} に書き出しました。ファイルには保有資産が含まれるので、取り扱いに注意してください。",
  },
  "Could not export: {reason}": { ja: "書き出せませんでした: {reason}" },
  "Loading…": { ja: "読み込み中…" },
  "Saved.": { ja: "保存しました。" },
  "Could not save: {reason}": { ja: "保存できませんでした: {reason}" },
  "Could not load: {reason}": { ja: "読み込めませんでした: {reason}" },
  "Enter a valid number.": { ja: "数値を入力してください。" },
  "Currency must be three letters, for example JPY.": {
    ja: "通貨は 3 文字で入力してください（例: JPY）。",
  },
  "Category targets add up to {sum}%, not 100%.": {
    ja: "カテゴリの目標の合計が {sum}% で、100% になっていません。",
  },
  "Category “{category}” has weighted products without a weight.": {
    ja: "カテゴリ「{category}」に、比率が未入力の商品があります。",
  },
  "Category “{category}” has no products.": {
    ja: "カテゴリ「{category}」に商品がありません。",
  },
  "Set the {currency}/JPY rate in Settings to value {currency} holdings.": {
    ja: "{currency} の保有を評価するには、設定で {currency}/JPY レートを入力してください。",
  },
  "Taxes are not taken into account in suggestions.": {
    ja: "提案では税金を考慮していません。",
  },
  "Relative weight inside the category": { ja: "カテゴリ内の相対比率" },
  "Amounts are in each product’s currency.": {
    ja: "金額は、各商品の通貨で入力します。",
  },
} as const satisfies Record<string, { ja: string }>;

export type MessageKey = keyof typeof MESSAGES;

let current: Lang = "en";

export function detectLanguage(saved: string | null, navigatorLanguage: string): Lang {
  if (saved === "en" || saved === "ja") {
    return saved;
  }
  return navigatorLanguage.toLowerCase().startsWith("ja") ? "ja" : "en";
}

export function setLanguage(lang: Lang): void {
  current = lang;
}

export function getLanguage(): Lang {
  return current;
}

/** Translates a message; `{name}` placeholders are replaced from `params`. */
export function t(key: MessageKey, params: Record<string, string> = {}): string {
  const template = current === "ja" ? MESSAGES[key].ja : key;
  return template.replace(/\{(\w+)\}/g, (whole, name: string) => params[name] ?? whole);
}
