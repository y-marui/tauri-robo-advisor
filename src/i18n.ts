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
  "Add from a preset": { ja: "プリセットから追加" },
  "Browser settings": { ja: "ブラウザの設定" },
  "No site": { ja: "サイトなし" },
  "These recipes name no site (an older version saved them), so they cannot be fetched. Delete them and save them again with a site.": {
    ja: "これらのレシピにはサイトがありません（古い版が保存したものです）。取得できないので、削除して、サイトを付けて保存し直してください。",
  },
  "Site the recipe reads (for example example.co.jp)": { ja: "レシピが読むサイト（例: example.co.jp）" },
  "Enter the site the recipe reads first.": { ja: "先に、レシピが読むサイトを入れてください。" },
  "For recipes that read cards, open the page that shows the holdings by hand after logging in.": {
    ja: "カードを読むレシピは、ログインしたあと、保有銘柄が見えるページを自分で開いてください。",
  },
  "Fetch all": { ja: "すべて取得" },
  "Recipe to add": { ja: "追加するレシピ" },
  "Choose a recipe and an account first.": { ja: "先にレシピと口座を選んでください。" },
  "Log in in the opened browser, then press Fetch again.": {
    ja: "開いたブラウザでログインしてから、もう一度「取得」を押してください。",
  },
  "You may not be logged in: log in in the browser, then press Fetch again.": {
    ja: "ログインしていない可能性があります。ブラウザでログインしてから、もう一度「取得」を押してください。",
  },
  "No login page is known for: {sites}. Log in and open a page of the site by hand, then press Fetch again.": {
    ja: "{sites} のログインのページは分かりません。自分でログインしてサイトのページを開いてから、もう一度「取得」を押してください。",
  },
  "No recipes yet. Add a built-in recipe below, or fetch a table once by hand in “Import a table” and use “Save as a recipe”.": {
    ja: "レシピはまだありません。下の組み込みレシピを追加するか、「表を取り込む」で一度手で取得して「レシピとして保存」してください。",
  },
  "Fetch this site": { ja: "このサイトを取得" },
  "Account of {recipe}": { ja: "{recipe} の口座" },
  "built-in": { ja: "組み込み" },
  "“Fetch” follows each recipe’s links (they only show other pages), reads its table, and shows the result. If a site has no open page, its login page is opened: log in by hand, then press Fetch again. Nothing changes until you confirm.": {
    ja: "「取得」は、各レシピのリンク（別のページを表示するだけのもの）をたどって表を読み、結果を表示します。サイトのページが開いていないときは、そのログインのページを開きます。自分でログインしてから、もう一度「取得」を押してください。確認するまで何も変わりません。",
  },
  "New account…": { ja: "新しい口座…" },
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
  "Group by": { ja: "まとめ方" },
  "By category": { ja: "カテゴリ別" },
  "Language": { ja: "言語" },
  "Categories": { ja: "カテゴリ" },
  "Records": { ja: "全レコード" },
  "Accounts": { ja: "口座" },
  "Holdings": { ja: "保有" },
  "Settings": { ja: "設定" },
  "Name": { ja: "名前" },
  "Target (%)": { ja: "目標 (%)" },
  "Category": { ja: "カテゴリ" },
  "Currency": { ja: "通貨" },
  "Type": { ja: "種別" },
  "Sellable": { ja: "売却可" },
  "Rounding unit": { ja: "丸め単位" },
  "Strategy": { ja: "戦略" },
  "Save strategy": { ja: "戦略を保存" },
  "Tick at least one product.": { ja: "商品を 1 つ以上選んでください。" },
  "Give a ratio to every ticked product, or leave them all blank.": { ja: "選んだ全商品に比率を入力するか、全商品を空欄にしてください。" },
  "Ratios must be positive.": { ja: "比率は 0 より大きい数にしてください。" },
  "Follow the portfolio": { ja: "ポートフォリオに従う" },
  "Specified products with ratios": { ja: "商品と比率を指定" },
  "Ratio of {product}": { ja: "{product} の比率" },
  "Tick the products and type a ratio for each (blank = equal share), then save. Category targets are not used for this account, and products outside the list are neither bought nor sold.": {
    ja: "商品にチェックを付け、比率を入力してください（空欄 = 等分。保存します。全商品を空欄にするか、全商品に入力します）。この口座ではカテゴリの目標を使わず、一覧にない商品は売買しません。",
  },
  "This account follows its own list of products and ratios. Category targets are not used, and products outside the list are neither bought nor sold.": {
    ja: "この口座は、自分の商品と比率の一覧に従います。カテゴリの目標は使わず、一覧にない商品は売買しません。",
  },
  "Total of the listed products: {amount} JPY": { ja: "一覧の商品の合計: {amount} 円" },
  "“{product}” is still {amount} JPY over its share.": { ja: "「{product}」はまだ割合より {amount} 円多いままです。" },
  "“{product}” is {amount} JPY short of its share.": { ja: "「{product}」は割合に {amount} 円足りません。" },
  "Add": { ja: "追加" },
  "Delete": { ja: "削除" },
  "Confirm delete": { ja: "削除する" },
  "Add a category": { ja: "カテゴリを追加" },
  "Add a product": { ja: "商品を追加" },
  "Add an account": { ja: "口座を追加" },
  "Add a category first.": { ja: "先にカテゴリを追加してください。" },
  "Add a record": { ja: "レコードを追加" },
  "No records": { ja: "レコードはありません" },
  "Filter products": { ja: "商品を絞り込む" },
  "Delete product": { ja: "商品を削除" },
  "Select": { ja: "選択" },
  "Select all shown": { ja: "表示中をすべて選択" },
  "Select none": { ja: "選択を解除" },
  "Select the products of": { ja: "このカテゴリの商品を選択" },
  "Category of the selected products": { ja: "選択した商品のカテゴリ" },
  "New category...": { ja: "新しいカテゴリ..." },
  "New category name": { ja: "新しいカテゴリの名前" },
  "Apply to the selection": { ja: "選択した商品に設定" },
  "{count} selected": { ja: "{count} 件選択中" },
  "Merge or split": { ja: "統合・分割" },
  "Merge into": { ja: "統合先" },
  "Merge": { ja: "統合" },
  "Record in both": { ja: "両方にあるレコード" },
  "Keep the record of the other product": { ja: "統合先のレコードを残す" },
  "Take the record of this product": { ja: "この商品のレコードを使う" },
  "Merging moves the aliases and the records to the other product and deletes this one.": {
    ja: "統合すると、別名とレコードが統合先に移り、この商品は削除されます。",
  },
  "Alias to split off": { ja: "切り出す別名" },
  "Name of the new product": { ja: "新しい商品の名前" },
  "Split off": { ja: "切り出す" },
  "Enter a name for the new product.": { ja: "新しい商品の名前を入力してください。" },
  "Splitting cuts an alias out as a new product. Tick the accounts whose records move with it.": {
    ja: "分割すると、別名が新しい商品として切り出されます。一緒に移すレコードの口座にチェックを入れてください。",
  },
  "Every product and every record, as saved. Edit here; the Holdings tab only shows them.": {
    ja: "保存されている商品とレコードのすべて。編集はここで行い、保有タブは表示だけです。",
  },
  "Category and group names are edited in the Categories and Groups cards.": {
    ja: "カテゴリとグループの名前は、「カテゴリ」と「グループ」のカードで編集します。",
  },
  "Enter an amount (0 is allowed). To remove a record, use its Delete button.": {
    ja: "金額を入力してください（0 も可）。レコードを消すには、その行の削除ボタンを使います。",
  },
  "Equal": { ja: "等分" },
  "Weight": { ja: "比率" },
  "{product}: weight ({mode})": { ja: "{product}: 比率（{mode}）" },
  "Enter a positive weight.": { ja: "比率には、0 より大きい数を入力してください。" },
  "The weight is set for the mode chosen above ({mode}); each mode keeps its own weight for a product, and every account tab and All keep their own. It splits a category's amount among its products, or the whole amount among all products when allocating by product weights.": {
    ja: "比率は、上で選んだ提案の種類（{mode}）のものです。種類ごとに、また口座のタブと「すべて」のタブごとに、商品の比率を別々に持ちます。「ポートフォリオに従う」ではカテゴリに配った額を商品の間で、「商品の比率で配る」では全体の額をすべての商品の間で、この比率で分けます。",
  },
  "General account": { ja: "一般" },
  "Specific account": { ja: "特定" },
  "NISA (unspecified)": { ja: "NISA（区分未定）" },
  "NISA growth": { ja: "NISA成長投資枠" },
  "NISA tsumitate": { ja: "NISAつみたて投資枠" },
  "Old NISA": { ja: "旧NISA" },
  "Crypto assets": { ja: "暗号資産" },
  "Not set": { ja: "未設定" },
  "Account types": { ja: "口座の種類" },
  "Add a type": { ja: "種類を追加" },
  "Type name": { ja: "種類の名前" },
  "Delete type": { ja: "種類を削除" },
  "Each record has a type (general, specific, NISA, NISA growth, NISA tsumitate, old NISA, cash, crypto assets, or one added here). “NISA” means a NISA whose frame is not decided yet. The type is read automatically only where the page shows it; otherwise set it by hand in the Records card.": {
    ja: "レコードごとに種類（一般・特定・NISA（区分未定）・NISA成長投資枠・NISAつみたて投資枠・旧NISA・現金・暗号資産、またはここで追加したもの）を持ちます。「NISA」は、成長投資枠かつみたて投資枠かをまだ決めていない NISA の意味です。種類を自動で読み取れるのは、ページに種類が表示されている場合だけです。それ以外は、レコードのカードで手で設定してください。",
  },
  "The sellable box says whether a suggestion may sell records of the type. NISA types are off by default, because selling gives up tax-free room. When a product is held in several types of one account, a sale takes from the other types first and from the NISA types last.": {
    ja: "「売却可」で、この種類のレコードを提案で売ってよいかを決めます。NISA 系は、売ると非課税枠を失うため、初期値では売りません。同じ口座の1つの商品が複数の種類にまたがる場合は、NISA 系以外の種類から先に売り、NISA 系は最後にします。",
  },
  "Built-in types": { ja: "標準の種類" },
  "Types you added": { ja: "追加した種類" },
  "Cash": { ja: "現金" },
  "USD/JPY rate": { ja: "USD/JPY レート" },
  "Category tolerance (%)": { ja: "カテゴリの許容乖離 (%)" },
  "Product tolerance (%)": { ja: "商品の許容乖離 (%)" },
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
  "Set the {currency}/JPY rate in Settings to value {currency} holdings.": {
    ja: "{currency} の保有を評価するには、設定で {currency}/JPY レートを入力してください。",
  },
  "Taxes are not taken into account in suggestions.": {
    ja: "提案では税金を考慮していません。",
  },
  "Amounts are in each product’s currency.": {
    ja: "金額は、各商品の通貨で入力します。",
  },
  "Overall balance": { ja: "全体のバランス" },
  "Target vs current": { ja: "目標と現在" },
  "Composition": { ja: "構成" },
  "Target": { ja: "目標" },
  "Current": { ja: "現在" },
  "No holdings yet.": { ja: "保有がまだありません。" },
  "Rebalance": { ja: "リバランス" },
  "This account has its own strategy (specified products with ratios). It takes no part in All; use its own tab.": {
    ja: "この口座は独自の戦略（商品と比率を指定）を持ちます。「すべて」には加わらないので、この口座のタブで提案してください。",
  },
  "Sell allowed": { ja: "売却を許可" },
  "Takes part": { ja: "売買に加える" },
  "{product} takes part in trading": { ja: "{product} を売買に加える" },
  "Show all": { ja: "すべて表示" },
  "Tradable only": { ja: "売買に加えるものだけ" },
  "There are no products yet. Add one in Settings.": { ja: "商品がまだありません。設定で追加してください。" },
  "No product takes part in trading.": { ja: "売買に加える商品はありません。" },
  "No products are held yet.": { ja: "保有している商品がありません。" },
  "Account settings": { ja: "口座の設定" },
  "Only products the account holds are listed. Ticked products take part in trading in their account. All and each account keep their own ticks. Products you leave out are never bought or sold, but still count toward their category. Tradable only lists just the ticked products.": {
    ja: "口座で保有している商品だけを表示します。チェックした商品が、その口座の売買に加わります。「すべて」と口座のタブは、それぞれ別にチェックを持ちます。外した商品は買いも売りもしませんが、カテゴリの保有額には数えます。「売買に加えるものだけ」は、チェックした商品だけを表示します。",
  },
  "Ticked products take part in trading in this account. Products you leave out are never bought or sold, but still count toward their category. A product the account does not hold is not bought. Tradable only lists just the ticked products.": {
    ja: "チェックした商品が、この口座の売買に加わります。外した商品は買いも売りもしませんが、カテゴリの保有額には数えます。口座で保有していない商品は買いません。「売買に加えるものだけ」は、チェックした商品だけを表示します。",
  },
  "Select all": { ja: "すべて選択" },
  "Clear": { ja: "すべて解除" },
  "All": { ja: "すべて" },
  "Suggestion mode": { ja: "提案の種類" },
  "Allocation method": { ja: "配り方" },
  "By product weights across categories": { ja: "商品の比率で配る（カテゴリをまたぐ）" },
  "Add a fixed amount": { ja: "定額追加" },
  "Amount needed to reach the target without selling": { ja: "売らずに達成する追加額" },
  "Full rebalance": { ja: "フル・リバランス" },
  "Sell a fixed amount": { ja: "定額売却" },
  "Amount (JPY)": { ja: "金額 (円)" },
  "Suggest": { ja: "提案する" },
  "Calculating…": { ja: "計算中…" },
  "Could not calculate: {reason}": { ja: "計算できませんでした: {reason}" },
  "Enter a positive amount.": { ja: "正の金額を入力してください。" },
  "Total": { ja: "合計" },
  "Change": { ja: "増減" },
  "After": { ja: "実行後" },
  "Product": { ja: "商品" },
  "Applied: {amount} JPY": { ja: "実行額: {amount} 円" },
  "Needed to reach the target: {amount} JPY": { ja: "目標に届くまでに必要な額: {amount} 円" },
  "Trades not listed in the table above": { ja: "上の表にない売買" },
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
  "By type": { ja: "種別ごと" },
  "Show type": { ja: "表示する種別" },
  "All types": { ja: "すべての種別" },
  "No accounts yet.": { ja: "口座がまだありません。" },
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
  "Create the API key with the permission to read balances only. No trading or withdrawal permission is needed. If you can, also restrict the key to your IP address.": {
    ja: "API キーには、残高の取得の権限だけを付けて作成してください。取引や出金の権限は不要です。可能なら、キーの利用元を自分の IP アドレスに限ってください。",
  },
  "Balances": { ja: "残高" },
  "Account to update": { ja: "更新する口座" },
  "Fetch balances": { ja: "残高を取得" },
  "Fetching…": { ja: "取得中…" },
  "Opening…": { ja: "開いています…" },
  "Could not fetch: {reason}": { ja: "取得できませんでした: {reason}" },
  "A product with this name already exists. Link to it instead.": {
    ja: "同じ名前の商品がすでにあります。その商品に対応づけてください。",
  },
  "A product with this name already exists.": { ja: "同じ名前の商品がすでにあります。" },
  "A product with this name exists: {name}": { ja: "同じ名前の商品があります: {name}" },
  "A product with a similar name exists: {name}": { ja: "似た名前の商品があります: {name}" },
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
  Prices: { ja: "価格" },
  "Prices: Lightning last price": { ja: "価格: Lightning の最終取引価格" },
  "Prices: bitFlyer price page": { ja: "価格: bitFlyer の価格ページ" },
  "Open the price page": { ja: "価格ページを開く" },
  "Open the price page, wait until the prices are shown, then fetch the balances: the prices are read from that page. The app only shows the page in the browser and reads it; it does not connect to it.": {
    ja: "価格ページを開き、価格が表示されてから残高を取得してください。価格はそのページから読みます。アプリはページをブラウザで表示して読むだけで、自分では接続しません。",
  },
  "Opened. Wait until the prices are shown, then fetch the balances.": {
    ja: "開きました。価格が表示されてから、残高を取得してください。",
  },
  "Could not open the page: {reason}": { ja: "ページを開けませんでした: {reason}" },
  "Not on the price page, so these could not be valued and are left out: {names}": {
    ja: "価格ページにないため評価できず、除外します: {names}",
  },
  "Page price vs Lightning last price (for reference)": {
    ja: "ページの価格と Lightning の最終取引価格（参考）",
  },
  "Page price": { ja: "ページの価格" },
  "Lightning price": { ja: "Lightning の価格" },
  Difference: { ja: "差" },
  "No JPY market, so these could not be valued and are left out: {names}": {
    ja: "円の市場がなく評価できないため、除外します: {names}",
  },
  "No yen rate, so these could not be valued and are left out: {names}": {
    ja: "円のレートがなく評価できないため、除外します: {names}",
  },
  "Values use GMO Coin's sales desk (hanbaijo) selling price.": {
    ja: "評価額は、GMO コインの販売所の売値を使っています。",
  },
  "No sales desk price exists for these, so they use the exchange's last executed price instead: {names}": {
    ja: "販売所の価格がない銘柄のため、取引所の最終約定価格を使っています: {names}",
  },
  "Create the API key with the permission to read the asset balance only. No trading, deposit or withdrawal permission is needed. If you can, also restrict the key to your IP address.": {
    ja: "API キーには、資産残高の参照の権限だけを付けて作成してください。取引・入出金の権限は不要です。可能なら、キーの利用元を自分の IP アドレスに限ってください。",
  },
  "Values use Binance's last traded price (exchange order book, not a sales desk price): the JPY market when there is one.": {
    ja: "評価額は、Binance の最終取引価格（取引所の板。販売所の価格ではありません）を使っています。円の市場がある銘柄は、その市場の価格です。",
  },
  "No JPY market exists for these, so they use the USDT market's last price times USDT/JPY: {names}": {
    ja: "円の市場がないため、USDT 市場の最終価格に USDT/JPY を掛けて評価しています: {names}",
  },
  "Create the API key with read-only permission. Do not enable trading, withdrawal or any other permission. If you can, also restrict the key to your IP address.": {
    ja: "API キーは、読み取り専用の権限で作成してください。取引・出金などの権限は付けないでください。可能なら、キーの利用元を自分の IP アドレスに限ってください。",
  },
  "Binance Japan accounts: this assumes your Binance Japan key also works on the global Binance API (api.binance.com). That is not confirmed by Binance. If the key is rejected, the message from Binance is shown above.": {
    ja: "Binance Japan のアカウントの場合: Binance Japan のキーが、グローバル版の Binance API（api.binance.com）でも使えることを前提にしています。Binance による確認はありません。キーが拒否されたときは、Binance のメッセージを上に表示します。",
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
  "kind not in the rules": { ja: "種別が規則にない" },
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
  "Log in to your brokerage in that browser, then fetch the recipes.": {
    ja: "そのブラウザで証券会社にログインしてから、レシピを取得してください。",
  },
  "No web page is open in the browser yet.": { ja: "ブラウザには、まだウェブページが開かれていません。" },
  "Open the holdings page in the browser first.": { ja: "先に、ブラウザで保有一覧のページを開いてください。" },
  "Reading the page…": { ja: "ページを読み取り中…" },
  "{rows} rows × {columns} columns: {label}": { ja: "{rows} 行 × {columns} 列: {label}" },
  "The page has more headings than the app reads, so card lists below the first ones are not offered. Choose the table with the holdings, then use it.": { ja: "このページには、アプリが読める数より多くの見出しがあるため、あとのほうのカード一覧は候補に出ません。保有が載っている表を選んで、使ってください。" },
  "{rows} cards × {columns} lines: {label}": { ja: "カード {rows} 件 × {columns} 行: {label}" },
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
  "Starts from the top page of Rakuten Securities (after logging in): follows \"保有商品一覧-すべて\" and reads the table of all holdings (units x price per 10,000 units). Investment trusts (units x price per 10,000 units) and domestic stocks (the evaluation) in yen are covered; other kinds are left out and reported, and so are holdings in a foreign currency.": {
    ja: "ログイン後の楽天証券のホームのページから始め、「保有商品一覧-すべて」を辿って、すべての保有の表を読みます。円建ての投資信託（口数 × 1万口あたりの基準価額）と国内株式（評価額）が対象で、ほかの種別は読まずに知らせます。外貨建ても対象外です。",
  },
  "After logging in to Daiwa Connect Securities, clicks the menu 国内株式 > 保有銘柄一覧 (the list of holdings, 現物取引保有銘柄一覧) and reads every holding row (the evaluation, with the quantity and price, which are checked against it).": {
    ja: "大和コネクト証券にログインしたあと、メニューの「国内株式」の「保有銘柄一覧」（現物取引保有銘柄一覧）をクリックして開き、保有の各行を読みます（評価額を読み、数量と現在値は評価額との照合に使います）。",
  },
  "{name}: read {value}, quantity x price {computed}": {
    ja: "{name}: 読んだ評価額 {value}、数量 × 価格 {computed}",
  },
  "{count} rows have an evaluation that differs from quantity x price (the evaluation is used):": {
    ja: "{count} 行で、評価額が 数量 × 価格 と合いません（評価額を使います）:",
  },
  "After logging in to Daiwa Connect Securities, clicks the menu 資産照会 > 取引余力・預り資産 and reads the yen balance (金銭残高).": {
    ja: "大和コネクト証券にログインしたあと、メニューの「資産照会」の「取引余力・預り資産」をクリックして開き、円の残高（金銭残高）を読みます。",
  },
  "Daiwa Connect Securities: domestic stocks": { ja: "大和コネクト証券: 国内株式" },
  "Open the list of holdings (現物取引保有銘柄一覧) of Daiwa Connect Securities after logging in, then fetch: every holding row is read (evaluation and quantity).": {
    ja: "ログイン後に、大和コネクト証券の「現物取引保有銘柄一覧」を開いてから取得します。保有の各行（評価額と数量・現在値）を読みます。",
  },
  "No login page is known for: {sites}.": { ja: "ログインのページが分かりません: {sites}。" },
  "Category of the new product": { ja: "新しい商品のカテゴリ" },
  "Currency of the new product": { ja: "新しい商品の通貨" },
  "Create a product and link": { ja: "商品を作って結びつける" },
  "To make a product from a name, add a category first (in the Categories card).": {
    ja: "名前から商品を作るには、先に「カテゴリ」のカードでカテゴリを追加してください。",
  },
  "Edit in Records": { ja: "全レコードで編集" },
  "Product name": { ja: "商品の名前" },
  "Daiwa Connect Securities: cash (yen)": { ja: "大和コネクト証券: 現金（円）" },
  "Open the page 取引余力・預り資産 of Daiwa Connect Securities after logging in, then fetch: the yen balance (金銭残高) is read.": {
    ja: "ログイン後に、大和コネクト証券の「取引余力・預り資産」のページを開いてから取得します。円の残高（金銭残高）を読みます。",
  },
  "PayPay Securities: holdings (provisional)": { ja: "PayPay 証券: 保有銘柄（暫定）" },
  "Reads the cards under 保有銘柄 of PayPay Securities (evaluation only: the page shows no quantity or price). After logging in, open the page that shows the holdings by hand (it is a web app that needs a click on the mini app entry, which this app does not make), then fetch.": {
    ja: "PayPay 証券の「保有銘柄」の下のカードを読みます（評価額だけ。ページには数量も価格も出ません）。ログイン後、保有が出るページを手で開いてから取得してください（Web アプリで、ミニアプリの項目のクリックが要りますが、このアプリはクリックしません）。",
  },
  "Move up": { ja: "上へ" },
  "Move down": { ja: "下へ" },
  "Drag to reorder": { ja: "ドラッグして並べ替え" },
  "“{recipe}” refers to a preset that this version of the app does not have. Remove the recipe.": {
    ja: "「{recipe}」は、このバージョンのアプリにない組み込みのレシピを指しています。レシピを削除してください。",
  },
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
  "Fetching “{recipe}”…": { ja: "「{recipe}」を取得中…" },
  "“{recipe}” failed: {reason}": { ja: "「{recipe}」に失敗しました: {reason}" },
  "Apply to the accounts": { ja: "口座に反映する" },
  "Confirm apply": { ja: "反映する" },
  "The holdings were replaced.": { ja: "保有を置き換えました。" },
  "Could not apply: {reason}": { ja: "反映できませんでした: {reason}" },
  "This replaces every holding of these accounts with the linked rows above: {accounts}. An account without any linked row is left as it is.": {
    ja: "次の口座の保有を、上の対応づけ済みの行ですべて置き換えます: {accounts}。対応づけ済みの行が1つもない口座は、そのままにします。",
  },
  "(this page)": { ja: "（このページ）" },
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
