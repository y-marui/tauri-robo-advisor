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
  "Overall balance": { ja: "全体のバランス" },
  "Target vs current": { ja: "目標と現在" },
  "Composition": { ja: "構成" },
  "Target": { ja: "目標" },
  "Current": { ja: "現在" },
  "No holdings yet.": { ja: "保有がまだありません。" },
  "Suggestion": { ja: "提案" },
  "Suggestion mode": { ja: "提案の種類" },
  "Add a fixed amount": { ja: "定額追加" },
  "Amount needed to reach the target without selling": { ja: "売らずに達成する追加額" },
  "Full rebalance": { ja: "フル・リバランス" },
  "Sell a fixed amount": { ja: "定額売却" },
  "Amount (JPY)": { ja: "金額 (円)" },
  "Suggest": { ja: "提案する" },
  "Calculating…": { ja: "計算中…" },
  "Could not calculate: {reason}": { ja: "計算できませんでした: {reason}" },
  "Enter a positive amount.": { ja: "正の金額を入力してください。" },
  "Choose the rebalancing account first.": { ja: "先に全体リバランスの対象口座を選んでください。" },
  "Total": { ja: "合計" },
  "Change": { ja: "増減" },
  "After": { ja: "実行後" },
  "Product": { ja: "商品" },
  "Applied: {amount} JPY": { ja: "実行額: {amount} 円" },
  "Needed to reach the target: {amount} JPY": { ja: "目標に届くまでに必要な額: {amount} 円" },
  "Trades in {account}": { ja: "{account} での売買" },
  "The amount was lowered from {requested} to {applied} JPY to fit the rounding unit.": {
    ja: "丸め単位に合わせて、{requested} 円から {applied} 円に下げました。",
  },
  "“{category}” is {amount} JPY short of its target.": {
    ja: "「{category}」は、目標に {amount} 円届きません。",
  },
  "“{category}” is still {amount} JPY over its target.": {
    ja: "「{category}」は、まだ目標を {amount} 円上回っています。",
  },
  "Every category is already within tolerance.": {
    ja: "すべてのカテゴリが許容乖離の範囲内です。",
  },
  "Holdings by category": { ja: "カテゴリ別の保有" },
  "Account": { ja: "口座" },
  "Amount": { ja: "金額" },
  "Value (JPY)": { ja: "評価額 (円)" },
  "Share": { ja: "構成比" },
  "Difference (points)": { ja: "目標との差 (ポイント)" },
  "By account": { ja: "口座ごと" },
  "Balance inside the account": { ja: "口座内の商品バランス" },
  "Add to this account": { ja: "この口座への追加" },
  "Short (+) / over (-)": { ja: "不足 (+) / 超過 (-)" },
  "No accounts yet.": { ja: "口座がまだありません。" },
  "Nothing held or buyable in this account.": { ja: "この口座には、保有も買える商品もありません。" },
  "Choose an account first.": { ja: "先に口座を選んでください。" },
  "API key": { ja: "API キー" },
  "API secret": { ja: "API シークレット" },
  "Save key": { ja: "キーを保存" },
  "No key is saved.": { ja: "キーは保存されていません。" },
  "A key is saved in the system keychain. It cannot be shown again.": {
    ja: "キーはシステムのキーチェーンに保存されています。再表示はできません。",
  },
  "Enter both the API key and the secret.": { ja: "API キーとシークレットの両方を入力してください。" },
  "Create the API key with the balance-read permission only. No trading or withdrawal permission is needed.": {
    ja: "API キーには、資産残高の取得の権限だけを付けて作成してください。取引や出金の権限は不要です。",
  },
  "Balances": { ja: "残高" },
  "Account to update": { ja: "更新する口座" },
  "Fetch balances": { ja: "残高を取得" },
  "Fetching…": { ja: "取得中…" },
  "Could not fetch: {reason}": { ja: "取得できませんでした: {reason}" },
  "Link each name to a product. The choice is remembered.": {
    ja: "名前ごとに商品を選んでください。選んだ対応は記憶されます。",
  },
  "Link": { ja: "対応づける" },
  "Unlink": { ja: "対応づけを外す" },
  "Add a {currency} product first.": { ja: "先に、{currency} 建ての商品を追加してください。" },
  "Linked to a product that is not in {currency}, so they are left out: {names}": {
    ja: "{currency} 建てでない商品に対応づけられているため、除外します: {names}",
  },
  "Value ({currency})": { ja: "評価額 ({currency})" },
  "No JPY market, so these could not be valued and are left out: {names}": {
    ja: "円の市場がなく評価できないため、除外します: {names}",
  },
  "Replace holdings": { ja: "保有を置き換える" },
  "Confirm replace": { ja: "置き換える" },
  "This replaces every holding of “{account}” with the linked rows above.": {
    ja: "「{account}」の保有を、上の対応づけ済みの行ですべて置き換えます。",
  },
  "Import a table": { ja: "表の取り込み" },
  "Log in to your brokerage in your own browser, then save the holdings as a CSV file or copy the table, and load it here.": {
    ja: "証券会社には自分のブラウザでログインし、保有を CSV で保存するか、表をコピーして、ここに読み込んでください。",
  },
  "Source name": { ja: "取り込み元の名前" },
  "Currency of the values": { ja: "値の通貨" },
  "Choose a file": { ja: "ファイルを選ぶ" },
  "Pasted table": { ja: "貼り付けた表" },
  "Or paste the table here": { ja: "または、表をここに貼り付けてください" },
  "Read pasted text": { ja: "貼り付けを読み込む" },
  "Name column": { ja: "名前の列" },
  "Value column": { ja: "評価額の列" },
  "Heading rows": { ja: "見出しの行数" },
  "Column": { ja: "列" },
  "Save columns": { ja: "列の対応を保存" },
  "Preview import": { ja: "取り込みを確認" },
  "Reading…": { ja: "読み込み中…" },
  "Could not read: {reason}": { ja: "読み込めませんでした: {reason}" },
  "The file is too large.": { ja: "ファイルが大きすぎます。" },
  "Paste a table first.": { ja: "先に表を貼り付けてください。" },
  "Enter a source name and choose the columns first.": {
    ja: "先に、取り込み元の名前と列を指定してください。",
  },
  "{rows} rows, {columns} columns": { ja: "{rows} 行、{columns} 列" },
  "{count} rows read": { ja: "{count} 行を読み取りました" },
  "{count} rows were left out: {rows}": { ja: "{count} 行を除外しました: {rows}" },
  "row": { ja: "行" },
  "column missing": { ja: "列がない" },
  "no name": { ja: "名前がない" },
  "not a number": { ja: "数値でない" },
  "negative or too large": { ja: "負の値、または大きすぎる" },
  "The value is": { ja: "評価額の取り方" },
  "A column": { ja: "1つの列" },
  "Quantity × price": { ja: "数量 × 価格" },
  "Quantity column": { ja: "数量の列" },
  "Price column": { ja: "価格の列" },
  "Price is quoted per": { ja: "価格の単位" },
  "1 unit": { ja: "1 単位" },
  "10,000 units": { ja: "1 万単位（投信の基準価額など）" },
  "Rows per record": { ja: "1件あたりの行数" },
  "Row with the name": { ja: "名前がある行" },
  "Row with the numbers": { ja: "数値がある行" },
  "Fetch with the browser (optional)": { ja: "ブラウザで取得（任意）" },
  "Opens Chrome, Edge, or Brave with a profile of its own. You log in yourself; the app only reads the tables on the page you open.": {
    ja: "Chrome・Edge・Brave を、専用のプロファイルで開きます。ログインはご自分で行い、アプリは開いたページの表を読むだけです。",
  },
  "No Chrome, Edge, or Brave was found on this computer.": {
    ja: "Chrome・Edge・Brave が、このコンピューターで見つかりませんでした。",
  },
  "Browser": { ja: "ブラウザ" },
  "Profile folder": { ja: "プロファイルのフォルダ" },
  "Profile folder (empty: inside the app's data folder)": {
    ja: "プロファイルのフォルダ（空: アプリのデータフォルダの中）",
  },
  "Open browser": { ja: "ブラウザを開く" },
  "Page": { ja: "ページ" },
  "Refresh pages": { ja: "ページを更新" },
  "Fetch tables": { ja: "表を取得" },
  "Table": { ja: "表" },
  "Use this table": { ja: "この表を使う" },
  "Close browser": { ja: "ブラウザを閉じる" },
  "(no title)": { ja: "（タイトルなし）" },
  "Opening the browser…": { ja: "ブラウザを開いています…" },
  "Log in to your brokerage in that browser, open the holdings page, then fetch the tables.": {
    ja: "そのブラウザで証券会社にログインし、保有一覧のページを開いてから、表を取得してください。",
  },
  "No web page is open in the browser yet.": { ja: "ブラウザには、まだウェブページが開かれていません。" },
  "Open the holdings page in the browser first.": { ja: "先に、ブラウザで保有一覧のページを開いてください。" },
  "Reading the page…": { ja: "ページを読み取り中…" },
  "{rows} rows × {columns} columns: {label}": { ja: "{rows} 行 × {columns} 列: {label}" },
  "No table was found on that page.": { ja: "そのページには、表が見つかりませんでした。" },
  "Choose the table with the holdings, then use it.": { ja: "保有が載っている表を選んで、使ってください。" },
  "The browser was closed.": { ja: "ブラウザを閉じました。" },
  "Data folder": { ja: "データの保存場所" },
  "Full path of a folder": { ja: "フォルダのフルパス" },
  "Use this folder": { ja: "このフォルダを使う" },
  "Use the iCloud Drive folder": { ja: "iCloud Drive のフォルダを使う" },
  "Use the local folder": { ja: "このコンピューターのフォルダに戻す" },
  "Use the data in that folder": { ja: "そのフォルダのデータを使う" },
  "Cancel": { ja: "キャンセル" },
  "That folder already has data. Use it as it is? The data on this computer stays where it is, as a backup.": {
    ja: "そのフォルダには、すでにデータがあります。そのまま使いますか？ このコンピューターのデータは、バックアップとしてそのまま残ります。",
  },
  "The data folder was changed.": { ja: "データの保存場所を変更しました。" },
  "Could not change the data folder: {reason}": { ja: "保存場所を変更できませんでした: {reason}" },
  "Enter the full path of a folder.": { ja: "フォルダのフルパスを入力してください。" },
  "The data is kept on this computer only: {folder}": { ja: "データはこのコンピューターだけに保存しています: {folder}" },
  "The data is kept in: {folder}": { ja: "データの保存場所: {folder}" },
  "A folder that your computer syncs (for example iCloud Drive) shares the data between your computers. The app sends nothing itself. The file holds your holdings, so choose a place you trust.": {
    ja: "コンピューターが同期するフォルダ（iCloud Drive など）を選ぶと、複数のコンピューターでデータを共有できます。アプリ自身は何も送りません。ファイルには保有資産が入るので、信頼できる場所を選んでください。",
  },
  "The data file is still being downloaded from the synced folder. Wait a moment, then come back to this window.": {
    ja: "データファイルが、同期フォルダからまだダウンロードされていません。しばらく待ってから、このウィンドウに戻ってください。",
  },
  "Other copies of the data file were found: {names}. Two computers saved at the same time; compare them and remove the ones you do not need.": {
    ja: "データファイルの別のコピーが見つかりました: {names}。2台が同時に保存したためです。内容を比べて、不要なものを削除してください。",
  },
  "A newer data file could not be used: {reason}": { ja: "新しいデータファイルを使えませんでした: {reason}" },
  "The data file was changed on another computer. It has been reloaded; please try again.": {
    ja: "データファイルが、別のコンピューターで変更されました。読み込み直したので、もう一度操作してください。",
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
