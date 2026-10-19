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
  "Groups": { ja: "グループ" },
  "Group": { ja: "グループ" },
  "Group name": { ja: "グループの名前" },
  "Group of the category": { ja: "カテゴリのグループ" },
  "Add a group": { ja: "グループを追加" },
  "Ungrouped": { ja: "グループなし" },
  "Groups sit above the categories. A group has no target of its own: it adds up its categories’ targets. Deleting a group keeps its categories.": {
    ja: "グループはカテゴリの上の分類です。グループ自身に目標はなく、カテゴリの目標を合計します。グループを削除しても、カテゴリは残ります。",
  },
  "Groups (inner) and categories (outer)": { ja: "グループ（内側）とカテゴリ（外側）" },
  "Fetch": { ja: "取得" },
  "What to fetch": { ja: "取得する対象" },
  "Everything": { ja: "すべて" },
  "Account: {name}": { ja: "口座: {name}" },
  "Site: {name}": { ja: "サイト: {name}" },
  "Fetch this recipe": { ja: "このレシピだけ取得" },
  "Fetch this recipe again": { ja: "このレシピをもう一度取得" },
  "Worked: {names}.": { ja: "取得できたもの: {names}。" },
  "Needs attention: {names}.": { ja: "対応が必要なもの: {names}。" },
  "Not tried: {names}.": { ja: "試していないもの: {names}。" },
  "Open the page that shows the table starting with “{title}” in the browser, then fetch this recipe again.": {
    ja: "「{title}」で始まる表があるページをブラウザで開いてから、このレシピをもう一度取得してください。",
  },
  "“{recipe}” ({account}) needs an open page of {site}: log in and open it in the browser first.": {
    ja: "「{recipe}」（{account}）には、{site} のページが開いている必要があります。ブラウザでログインして、ページを開いてください。",
  },
  "“{recipe}” ({account}) failed: {reason}": { ja: "「{recipe}」（{account}）が失敗しました: {reason}" },
  "Add an account first (below), then add the preset.": {
    ja: "先に口座を追加してください（下の欄）。そのあとでレシピを追加します。",
  },
  "Name of a new account": { ja: "新しい口座の名前" },
  "New account name": { ja: "新しい口座の名前" },
  "Enter a name for the new account.": { ja: "新しい口座の名前を入力してください。" },
  "Import": { ja: "取り込み" },
  "Main sections": { ja: "メインの区分" },
  "Preset import: steps": { ja: "組み込みのレシピで取り込む手順" },
  "Done": { ja: "済" },
  "To do": { ja: "未" },
  "Add a category and an account (Settings tab).": { ja: "カテゴリと口座を追加します（設定タブ）。" },
  "Choose a preset and an account, and add it (Fetch recipes card).": {
    ja: "組み込みのレシピと口座を選んで追加します（取得レシピのカード）。",
  },
  "Open the browser and the login pages, and log in yourself (Fetch recipes card).": {
    ja: "ブラウザとログインのページを開き、ログインは自分で行います（取得レシピのカード）。",
  },
  "Fetch (Fetch recipes card).": { ja: "取得します（取得レシピのカード）。すべて、口座ごと、サイトごと、レシピ 1 つずつのどれでも選べます。" },
  "Link the names to products (one click creates a product), then apply.": {
    ja: "名前を商品に結び付け（1クリックで商品を作れます）、反映します。",
  },
  "A table from a file, pasted text, or the browser, with your own columns, is in the Import a table card below.": {
    ja: "ファイル、貼り付け、ブラウザの表を、自分で列を選んで取り込む方法は、下の「表の取り込み」のカードにあります。",
  },
  "The file is left untouched. To start a new file, move portfolio.json out of the data folder and open the app again.": {
    ja: "ファイルはそのまま残してあります。新しく始めるには、データの保存場所から portfolio.json を別の場所へ移して、アプリを開き直してください。",
  },
  "No value yet": { ja: "評価額はまだありません" },
  "Group by": { ja: "まとめ方" },
  "By category": { ja: "カテゴリ別" },
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
  "Relative weight inside the category: {account}": { ja: "カテゴリ内の相対比率: {account}" },
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
  "Suggestions": { ja: "提案" },
  "Holdings sections": { ja: "保有の区分" },
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
  "Quantity": { ja: "数量" },
  "Price": { ja: "価格" },
  "Quantity column to record": { ja: "記録する数量の列" },
  "none": { ja: "なし" },
  "(the product’s currency)": { ja: "（商品の通貨）" },
  "Value (JPY)": { ja: "評価額 (円)" },
  "Share": { ja: "構成比" },
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
  "Add a product first.": { ja: "先に商品を追加してください。" },
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
  "Value of the name cell": { ja: "名前のセルの何番目の値か（0から）" },
  "Value of the number cell": { ja: "数値のセルの何番目の値か（0から）" },
  "Value of the price cell": { ja: "価格のセルの何番目の値か（0から）" },
  "whole cell": { ja: "セル全体" },
  "Allow text around numbers": { ja: "数値の前後の文字を許す" },
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
  "SBI Securities: domestic stocks": { ja: "SBI証券: 国内株式" },
  "SBI Securities: investment trusts": { ja: "SBI証券: 投資信託" },
  "Starts from the top page of SBI Securities (after logging in): follows \"保有資産評価\" and reads the domestic stock tables of every account type (quantity x current price).": {
    ja: "ログイン後のSBI証券のトップのページから始め、「保有資産評価」を辿って、口座の種類ごとの国内株式の表をすべて読みます（数量 × 現在値）。",
  },
  "Starts from the top page of SBI Securities (after logging in): follows \"保有資産評価\" and reads the investment trust tables of every account type (units x price per 10,000 units).": {
    ja: "ログイン後のSBI証券のトップのページから始め、「保有資産評価」を辿って、口座の種類ごとの投資信託の表をすべて読みます（口数 × 1万口あたりの基準価額）。",
  },
  "“{recipe}” needs an open page of {site}: log in and open it in the browser first.": {
    ja: "「{recipe}」には、{site} の開いているページが要ります。ブラウザでログインして、先にそのページを開いてください。",
  },
  "Monex Securities: investment trusts": { ja: "マネックス証券: 投資信託" },
  "Starts from the top page of Monex Securities (after logging in): follows \"残高照会\" and reads the investment trust table (units x price per 10,000 units).": {
    ja: "ログイン後のマネックス証券のトップのページから始め、「残高照会」を辿って、投資信託の表を読みます（口数 × 1万口あたりの基準価額）。",
  },
  "Rakuten Securities: all holdings": { ja: "楽天証券: すべての保有" },
  "Starts from the top page of Rakuten Securities (after logging in): follows \"保有商品一覧-すべて\" and reads the table of all holdings (evaluation and units held). Only holdings in yen are covered, not holdings in a foreign currency such as foreign stocks.": {
    ja: "ログイン後の楽天証券のホームのページから始め、「保有商品一覧-すべて」を辿って、すべての保有の表を読みます（評価額と保有数）。対象は円建ての保有だけで、外国株などの外貨建ては対象外です。",
  },
  "After logging in to Daiwa Connect Securities, clicks the menu 国内株式 > 保有銘柄一覧 (the list of holdings, 現物取引保有銘柄一覧) and reads every holding row (evaluation and quantity).": {
    ja: "大和コネクト証券にログインしたあと、メニューの「国内株式」の「保有銘柄一覧」（現物取引保有銘柄一覧）をクリックして開き、保有の各行を読みます（評価額と数量）。",
  },
  "After logging in to Daiwa Connect Securities, clicks the menu 資産照会 > 取引余力・預り資産 and reads the yen balance (金銭残高).": {
    ja: "大和コネクト証券にログインしたあと、メニューの「資産照会」の「取引余力・預り資産」をクリックして開き、円の残高（金銭残高）を読みます。",
  },
  "Daiwa Connect Securities: domestic stocks": { ja: "大和コネクト証券: 国内株式" },
  "Open the list of holdings (現物取引保有銘柄一覧) of Daiwa Connect Securities after logging in, then fetch: every holding row is read (evaluation and quantity).": {
    ja: "ログイン後に、大和コネクト証券の「現物取引保有銘柄一覧」を開いてから取得します。保有の各行（評価額と数量）を読みます。",
  },
  "Open the login pages": { ja: "ログインのページを開く" },
  "Opened the login page of: {sites}. Log in by hand, then open the page you want.": {
    ja: "{sites} のログインのページを開きました。自分でログインしてから、読みたいページを開いてください。",
  },
  "Already open: {sites}.": { ja: "すでに開いています: {sites}。" },
  "No login page is known for: {sites}.": { ja: "ログインのページが分かりません: {sites}。" },
  "No recipe says which site it reads, so there is no login page to open.": {
    ja: "どのサイトを読むかが決まったレシピがないので、開くログインのページがありません。",
  },
  "Name of the new product": { ja: "新しい商品の名前" },
  "Category of the new product": { ja: "新しい商品のカテゴリ" },
  "Currency of the new product": { ja: "新しい商品の通貨" },
  "Create a product and link": { ja: "商品を作って結びつける" },
  "To make a product from a name, add a category first (in the Categories card).": {
    ja: "名前から商品を作るには、先に「カテゴリ」のカードでカテゴリを追加してください。",
  },
  "Category name": { ja: "カテゴリの名前" },
  "Product name": { ja: "商品の名前" },
  "Category of the product": { ja: "商品のカテゴリ" },
  "Daiwa Connect Securities: cash (yen)": { ja: "大和コネクト証券: 現金（円）" },
  "Open the page 取引余力・預り資産 of Daiwa Connect Securities after logging in, then fetch: the yen balance (金銭残高) is read.": {
    ja: "ログイン後に、大和コネクト証券の「取引余力・預り資産」のページを開いてから取得します。円の残高（金銭残高）を読みます。",
  },
  "Move up": { ja: "上へ" },
  "Move down": { ja: "下へ" },
  "Preset": { ja: "組み込みのレシピ" },
  "Add the preset": { ja: "追加する" },
  "Detach as a custom recipe": { ja: "自分のレシピにする" },
  "Makes an ordinary recipe of it with the definition it has now. It no longer follows app updates, and can be edited.": {
    ja: "今の定義のまま、普通のレシピにします。以後はアプリの更新に追従せず、編集できます。",
  },
  "built-in, read-only": { ja: "組み込み・編集不可" },
  "“{recipe}” refers to a preset that this version of the app does not have. Remove the recipe.": {
    ja: "「{recipe}」は、このバージョンのアプリにない組み込みのレシピを指しています。レシピを削除してください。",
  },
  "Choose a preset and an account first.": { ja: "先に、組み込みのレシピと口座を選んでください。" },
  "Fetch recipes": { ja: "取得レシピ" },
  "The recipe was changed or removed in the meantime. Fetch again.": {
    ja: "その間に、レシピが変更または削除されました。もう一度取得してください。",
  },
  "Where the link goes": { ja: "リンクの行き先" },
  "Choose the link again": { ja: "リンクを選び直す" },
  "The link “{text}” of “{recipe}” has changed since it was saved.": {
    ja: "レシピ「{recipe}」のリンク「{text}」が、保存したときから変わっています。",
  },
  "(the page handles it)": { ja: "（ページ内の処理）" },
  "Use this link": { ja: "このリンクを使う" },
  "Several different links say “{text}” (recipe “{recipe}”). Choose where it goes.": {
    ja: "「{text}」と書かれたリンクが、行き先の違うものとして複数あります（レシピ「{recipe}」）。行き先を選んでください。",
  },
  "Open the browser, log in, and open a page of the site first.": {
    ja: "先にブラウザを開き、ログインして、そのサイトのページを開いてください。",
  },
  "Fetching “{recipe}”…": { ja: "「{recipe}」を取得中…" },
  "“{recipe}” failed: {reason}": { ja: "「{recipe}」に失敗しました: {reason}" },
  "{count} rows were left out.": { ja: "{count} 行を除外しました。" },
  "Apply to the accounts": { ja: "口座に反映する" },
  "Confirm apply": { ja: "反映する" },
  "The holdings were replaced.": { ja: "保有を置き換えました。" },
  "Could not apply: {reason}": { ja: "反映できませんでした: {reason}" },
  "This replaces every holding of these accounts with the linked rows above: {accounts}. An account without any linked row is left as it is.": {
    ja: "次の口座の保有を、上の対応づけ済みの行ですべて置き換えます: {accounts}。対応づけ済みの行が1つもない口座は、そのままにします。",
  },
  "No recipes yet. Fetch a table once by hand in “Import a table”, then use “Save as a recipe”.": {
    ja: "レシピはまだありません。「表の取り込み」で一度手で表を取得してから、「レシピとして保存」を使ってください。",
  },
  "(this page)": { ja: "（このページ）" },
  "After you log in to the browser opened by the app, “Fetch” follows each recipe’s links (they only show other pages), reads its table, and shows the result. Choose everything, one account, or one site, or fetch a single recipe. Nothing changes until you confirm.": {
    ja: "アプリが開いたブラウザにログインしたあと、「取得」を押すと、各レシピのリンク（別のページを表示するだけのもの）をたどって表を読み、結果を見せます。すべて、1 つの口座、1 つのサイトのどれかを選ぶか、レシピを 1 つだけ取得できます。確認するまで、何も変わりません。",
  },
  "Save as a recipe": { ja: "レシピとして保存" },
  "Recipe name": { ja: "レシピの名前" },
  "Links to follow, one per line (empty: the open page)": { ja: "たどるリンク（1行に1つ。空なら今のページ）" },
  "Text the table’s first cell starts with": { ja: "表の先頭のセルの書き出し" },
  "Sites the recipe may follow links to (optional)": { ja: "リンクをたどってよいサイト（任意。例: sbisec.co.jp）" },
  "Save recipe": { ja: "レシピを保存" },
  "The recipe was saved.": { ja: "レシピを保存しました。" },
  "Choose an account and enter the recipe name and the table title first.": {
    ja: "先に、口座を選び、レシピの名前と表のタイトルを入れてください。",
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
  // A key the dictionary lacks shows as it is instead of breaking the whole screen.
  const template = current === "ja" ? (MESSAGES[key]?.ja ?? key) : key;
  return template.replace(/\{(\w+)\}/g, (whole, name: string) => params[name] ?? whole);
}
