# Crypto Exchanges Investigation

#122 の調査結果。Coincheck・GMO コイン・CoinTrade・SBI VC トレード・Binance の保有を、設計制約（ローカルのみ、認証情報はキーチェーンのみ、読み取り専用、注文をしない）の範囲で読めるかを、公式ドキュメントだけで調べた。調査日は 2026-10-20。本アプリは投資助言ではなく、ここに実際の残高やキーは含まれない。

## Summary

| 取引所 | 方式 | 認証 | 読み取り専用の権限 | 価格の取得元 | 難しさ | 推奨 |
|---|---|---|---|---|---|---|
| Coincheck | 公式 REST API | `ACCESS-KEY` / `ACCESS-NONCE`（単調増加）/ `ACCESS-SIGNATURE`（HMAC-SHA256 の hex） | 権限は機能ごとに設定する。公式ページには文字がなく画面の図のみ。第三者の連携サービスの手順は「残高」だけを許可している（実装時に画面で確認） | 公開 `GET /api/rate/xxx_jpy`（販売所のレート `rate`、文字列）。取引所の ticker（`/api/ticker?pair=`）には無い通貨（例: MATIC）も引ける。一括取得はなく通貨ごと | 小（bitFlyer とほぼ同じ形） | 1 番目 |
| GMO コイン | 公式 REST API | `API-KEY` / `API-TIMESTAMP`（ミリ秒）/ `API-SIGN`（時刻+メソッド+パス+本文の HMAC-SHA256、hex） | 権限は機能ごとに設定する。「参照のみ」に当たる項目があるかは、読めた範囲では確認できなかった（実装時に画面で確認） | 公開 `GET /public/v1/ticker`（`symbol` 省略で全銘柄を一度に取得。`last`）。残高の応答にも円換算率（`conversionRate`）がある | 小 | 2 番目 |
| SBI VC トレード | API なし（公式 FAQ は「今後公開を予定」。時期の記載なし） | - | - | ログイン後の画面の表 | 中（利用者が画面を確認してから） | レシピのプリセット。API が出るまで待たない |
| CoinTrade | API なし（公式サポートの FAQ に「APIはありますか」があり、提供なしと読める。取得時に 403 で全文は確認できず、検索結果の要約による） | - | - | 販売所の画面 | - | 見送り |
| Binance | 公式 REST API（`GET /api/v3/account`、`USER_DATA`） | `X-MBX-APIKEY` + 署名（HMAC-SHA256 / RSA / Ed25519） | キーの既定は取引不可。取引を許可しなければ読み取りのみ | 公開 `GET /api/v3/ticker/price`。ほとんどが USDT 建てで、円への換算に USDT/JPY が要る | 中（円換算、日本居住者向けの扱いの確認） | 最後。方針を決めてから |

## Per Exchange

### Coincheck

- 残高: `GET /api/accounts/balance`。通貨ごとに `<通貨>`、`<通貨>_reserved`、`_lending`、`_debt`、`_tsumitate` などの文字列の数値。評価に使う数量は、保有の合計（注文中の `_reserved` やつみたてを含めるか）を実装で決める。
- 制限: 公式に書かれている制限は、新規注文（秒 4 回）と注文の詳細（秒 1 回）のみ。残高の制限の記載はない。429 は応答コードで扱う。
- リスク: 署名の対象について、公式の例が一致していない（URL と本文の両方か、URL のみか）。GET は本文が空なので、`nonce + URL` で署名する。nonce は API キーごとに単調増加でなければならず、時刻（ミリ秒以上）を使う。IP 制限をキーに付けられる。
- 価格: 通貨ごとの販売所レート（`/api/rate/<通貨>_jpy`）を、保有のある通貨だけ叩く。利用者が実際に売買する価格なので、取引所の ticker ではなくこちらを使う（実機で、ticker に無い MATIC がレートでは引けると確認した）。`Invalid pair` の通貨は `unpriced`。

### GMO Coin

- 残高: `GET /private/v1/account/assets`。`symbol`、`amount`、`available`、`conversionRate`（売値ベースの円換算率）。
- 制限: 秒あたり 20 回（先週の取引量が 10 億円未満の段階。GET と POST は別枠）。このアプリの用途には十分。
- リスク: 署名は `timestamp + method + path + body`、パスは `/v1/...`（`/private` を含まない）。販売所と取引所の残高が同じ応答に入るかは、実装時に確認する。IP 制限をキーに付けられる。
- 価格: 全銘柄の ticker を 1 回で取れる。円換算率は売値ベースなので、他の取引所と同じ基準（`last`）で評価するかを実装で決める。

### SBI VC Trade

- 公式 FAQ は「今後、公開を予定」と答えるだけで、公開済みの API や仕様書は見つからなかった。API の方式は採れない。
- 画面の表を読むレシピなら、既存の「ブラウザで取得」で読める見込み。ただし、実際のページ構造は、ログインした利用者が確認する必要がある（既存のプリセットと同じ扱い）。確認できたら、プリセットをデータとして追加し、`sbivc.co.jp` を `portfolio::VETTED_SITES` に加える（別ホストへのリンクをたどる場合のみ。同じホストだけなら加えない）。
- 注文・入出金に見えるリンクは、既存の規則で押さない。

### CoinTrade

- 公式サポートの FAQ に API があるかの項目があり、提供されていないと読める。販売所のみのサービス。
- 保有は手入力か、既存の汎用レシピ・表の貼り付けで足りる。専用の対応を作る価値は低い。

### Binance

- 残高: `GET /api/v3/account`（重み 20、IP 単位の制限）。`balances[].asset / free / locked`。`omitZeroBalances=true` で 0 を除ける。
- 署名: HMAC-SHA256・RSA・Ed25519。公式は Ed25519 を勧める。`timestamp` と `recvWindow` が必須または推奨。
- 権限: キーの既定は取引不可で、取引や出金を許可しなければ読み取り専用になる。
- 価格: ほとんどが USDT・BUSD などの建て。円の評価は `USDTJPY` などの換算が要り、換算できない通貨は `unpriced`。bitFlyer の「同じ基準で全通貨を評価する」考え方から外れる。
- 未確認: 日本居住者が使う Binance（国際版）と Binance Japan は別のサービスで、API の入口や権限の画面が違う。Binance Japan の公式 API 仕様は、今回の調査では確認できなかった。どちらを対象にするかは利用者が決める。

## Fit with the bitFlyer Design

`crates/bitflyer` の構成（`Credentials`（`Debug` で秘匿）、`Transport` トレイト（テストでネットワーク不要）、`sign`、`Client`、`Holding`/`Holdings`、`unpriced`）を、取引所ごとの小さな crate で踏襲する。

- キーは `src-tauri/src/secrets.rs` を通して OS のキーチェーンだけに置く。通信は Rust の `reqwest`（既存）で、固定の `https` と GET のみ。リダイレクトは追従しない。
- 金額は十進数の文字列（`rust_decimal`）。新しい依存は、Binance の RSA / Ed25519 署名を使う場合のみ必要になる（HMAC-SHA256 なら既存の `hmac` / `sha2` で足りる）。その場合は依存の追加を Issue で先に承認してもらう。
- `Transport`・`Request`・`Response`・`Holding` は bitFlyer と同形の重複になる。2 つ目の API crate（GMO コイン）を作る時点で、共通部分の crate への抽出を検討する（`Extraction Policy`：条件を満たす前に分けない）。
- レシピ方式は新しい通信のコードを要せず、プリセットのデータとログインの案内だけで済む。

## Recommended Order

1. Coincheck（利用者が多く、通貨ごとの JPY 建てがあり、bitFlyer に最も近い）。
2. GMO コイン（ticker が一括で取れる。共通部分の抽出の判断をここで行う）。
3. SBI VC トレードのレシピのプリセット（利用者が画面の構造を確認できてから）。
4. Binance（方針の決定を待つ）。

見送り: CoinTrade（API がなく、販売所のみで、手入力か汎用の取り込みで足りる）。
