# File Map

_最終更新: 2026-10-19_

## Frontend

| ファイル | 役割 | 主な依存先 |
|---|---|---|
| `index.html` | HTML のエントリ | `src/main.ts`, `src/styles.css` |
| `src/main.ts` | アプリの起点。状態の保持、言語の切り替え、タブとパネルの組み立て | `src/api.ts`, `src/views/*` |
| `src/views/account-strategy.ts` | 設定の口座の「戦略」（ポートフォリオに従う / 商品と比率を指定）の編集（下書きを整合したときだけ保存） | `src/views/accounts.ts`, `src/strategy-draft.ts`, `src/types.ts` |
| `src/strategy-draft.ts` | 戦略の下書きの検証（商品あり・比率は全部空欄か全部入力・正の数）。保存の可否と理由を返す | `src/decimal.ts` |
| `src/api.ts` | Rust のコマンド（`get_portfolio`、`apply_portfolio_edit`、`get_overview`、`get_account_balance`、`suggest_rebalance`）の呼び出し | `@tauri-apps/api/core` |
| `src/types.ts` | Rust の JSON に対応する型（金額は十進数の文字列） | — |
| `src/decimal.ts` | 十進数の文字列の変換（％ ↔ 小数、桁区切り表示）。浮動小数点を使わない | — |
| `src/i18n.ts` | 画面の文言（英語がキー、日本語は翻訳） | — |
| `src/panel.ts`, `src/fields.ts`, `src/dom.ts` | パネルの描画、入力部品、DOM の補助 | `src/i18n.ts` |
| `src/views/holdings-table.ts` | 保有タブの表。カテゴリ別・口座別・種別ごとの一覧と種別での絞り込み、グループとカテゴリの小計（表示だけ。編集は全レコードのカード） | `src/panel.ts`, `src/views/groups.ts` |
| `src/views/account-types.ts` | 設定タブの「口座の種類」カード。追加した種別の追加・名前・売却可・削除 | `src/panel.ts`, `src/fields.ts` |
| `src/views/records.ts` | 設定タブの「全レコード」カード。商品とレコード（口座・商品・種別）の編集、レコードと商品の追加、商品名の絞り込み | `src/panel.ts`, `src/fields.ts`, `src/views/holdings-table.ts`（`priceText`） |
| `src/views/groups.ts` | グループのカード（追加・名前の変更・削除）と、グループの表示の順序・名前の補助 | `src/panel.ts`, `src/fields.ts` |
| `src/views/import-steps.ts` | 取り込みタブの、組み込みのレシピで取り込む手順（済・未の表示） | `src/panel.ts` |
| `src/subtabs.ts` | 保有タブの副タブの名前、保存した選択の検証、矢印キーの移動（テストあり） | — |
| `src/views/rebalance.ts` | 保有タブの副タブ「リバランス」。先頭の「すべて」（口座ごとの売買に加える商品の選択と、全口座をまたぐ提案）と口座ごとのタブ（口座内の商品バランス、口座への追加の提案） | `src/rebalance-selection.ts` | 「すべて」のタブの、売買に加える商品の選択（保有する商品、すべて選択・解除。`excluded` の裏返し） | `src/types.ts` |
| `src/views/suggestion.ts` |
| `src/views/suggestion.ts` | 提案の入力欄と結果の表示（「すべて」のタブと口座のタブで共有） | `src/api.ts` |
| `src/views/bitflyer.ts` | bitFlyer のカード。キーの保存・削除、残高の取得と対応づけ、保有の置き換え | `src/api.ts` |
| `src/views/browser-controls.ts` | ブラウザの選択・プロファイルのフォルダ・開く・閉じる（「ブラウザで取得」と取得レシピで共有。最後の選択を初期値にする） | `src/api.ts`, `src/browser-defaults.ts` |
| `src/views/browser-fetch.ts` | 「ブラウザで取得」。ページと表の選択（ブラウザの起動は `browser-controls.ts`） | `src/api.ts` |
| `src-tauri/src/browser_session.rs` | 専用ブラウザの接続先と、取得した表（とカード一覧）の保持。ページはホストとタイトルだけを返す。つなげた表の各表の位置とタイトル（`Section`）も返す。終了時に閉じる | `crates/browser`, `crates/tabular` |
| `src/views/data-folder.ts` | データの保存場所のカードと、同期に関する知らせ | `src/api.ts` |
| `src-tauri/src/browser_settings.rs` | 最後に使ったブラウザの種類とプロファイルのフォルダ（ローカルの `browser-settings.json`。ポートフォリオのデータには入れない） | — |
| `src-tauri/src/data_folder.rs` | 保存先のフォルダの選択（ポインタファイル）、競合のコピーと未ダウンロードの検出、iCloud Drive のフォルダ | — |
| `src/views/recipes.ts` | 取得レシピのカード（サイトごとの一覧、取得、口座の変更、ログインのページ、リンクの選択、反映） | `src/views/coincheck.ts` | Coincheck のカード。キーの保存・削除、残高の取得と対応づけ、保有の置き換え | `src/api.ts` |
| `src/views/gmocoin.ts` | GMO コインのカード。Coincheck と同じ流れで、価格の基準（販売所 / 取引所の最終価格）の注意を出す | `src/api.ts` |
| `src/views/binance.ts` | Binance のカード。Coincheck と同じ流れで、Binance Japan の前提（未確認）と価格の基準（JPY 市場 / USDT 経由）の注意を出す | `src/api.ts`, `src/exchange-account.ts` |
| `src/views/import-plan.ts` |
| `src/recipe-groups.ts` | 取得レシピのカードの純粋な補助（表示名、サイトごとの分け方、ログインが要るサイトの判定） | `src/i18n.ts`, `src/types.ts` |
| `crates/browser/src/links.rs` | リンクをたどる規則（押してよいリンク、行き先の参照、安全な文字列の受け渡し） | — |
| `src/views/table-import.ts` | 表の取り込みのカード（ファイル・貼り付け・列の指定・保存） | `src/views/import-plan.ts` |
| `src/views/import-plan.ts` | 取り込み結果の表示（対応づけ・別名の追加と解除・保有の置き換え）。bitFlyer・Coincheck・GMO コイン・Binance と表の取り込みで共有 | `src/api.ts` |
| `src/views/differences.ts` | 評価額が 数量 × 価格 と合わない行の文面（取り込みの確認の画面用） | `src/types.ts` |
| `src/views/*.ts` | カテゴリ・商品・口座・保有・設定の各パネル、全体のバランス（グラフと提案）のパネル、不備の表示 | `src/panel.ts`, `src/fields.ts` |
| `src/*.test.ts` | Node の組み込みテストランナーによる単体テスト（`npm test`） | — |
| `src/styles.css` | スタイル | — |

## Backend

| ファイル | 役割 | 主な依存先 |
|---|---|---|
| `src-tauri/src/main.rs` | デスクトップのエントリ | `src-tauri/src/lib.rs` |
| `src-tauri/src/lib.rs` | Tauri のセットアップ、`propose_rebalance`、ポートフォリオ（`get_portfolio`、`apply_portfolio_edit`、`get_overview`、`get_account_balance`、`suggest_rebalance`）と bitFlyer・Coincheck・GMO コイン・Binance 関連のコマンド | `tauri`, `crates/rebalance`, `crates/bitflyer`, `src-tauri/src/secrets.rs` |
| `src-tauri/src/store.rs` | ポートフォリオ JSON の読み書き（一時ファイル → 同期 → 置き換え、権限 0600、サイズ上限） | `crates/portfolio` |
| `src-tauri/src/app_state.rs` | メモリ上のポートフォリオ。編集を検証・保存してから反映する | `store.rs`, `crates/portfolio` |
| `src-tauri/src/secrets.rs` | OS キーチェーンへの API キーの保存・読み出し・削除と入力検証 | `keyring`, `crates/bitflyer`, `crates/coincheck`, `crates/gmocoin`, `crates/binance` |
| `src-tauri/tauri.conf.json` | アプリ設定 | — |
| `src-tauri/capabilities/default.json` | 権限定義 | — |

## Domain

| ファイル | 役割 | 主な依存先 |
|---|---|---|
| `Cargo.toml` | Cargo ワークスペース定義と共通のパッケージ情報 | — |
| `crates/rebalance/src/lib.rs` | リバランス計算 `propose`、入力検証、テスト | `rust_decimal`, `serde`, `thiserror` |
| `crates/bitflyer/src/client.rs` | `Client`：残高取得、JPY 評価、署名付きリクエスト生成 | `transport.rs`, `model.rs`, `sign.rs`, `rebalance` |
| `crates/bitflyer/src/page_prices.rs` | 価格ページの表を通貨ごとの円の価格に読む（`PagePrices`）。`Client::holdings_with_page_prices` が使う | `rust_decimal` |
| `crates/bitflyer/src/transport.rs` | `Transport` トレイトと HTTPS 実装（リダイレクトなし・サイズ上限） | `reqwest` |
| `crates/bitflyer/src/model.rs` | 応答の解析と検証（通貨コード・金額・エラーメッセージ） | `serde_json`, `rust_decimal` |
| `crates/bitflyer/src/sign.rs` | HMAC-SHA256 署名 | `hmac`, `sha2` |
| `crates/bitflyer/src/credentials.rs` | 値を表示しない認証情報の型 | — |
| `crates/bitflyer/src/error.rs` | 認証情報を含まないエラー型 | `thiserror` |
| `crates/coincheck/src/client.rs` | `Client`：残高取得、JPY 評価（公開ティッカー）、単調増加の nonce、署名付きリクエスト生成 | `transport.rs`, `model.rs`, `sign.rs` |
| `crates/coincheck/src/transport.rs` | `Transport` トレイトと HTTPS 実装（リダイレクトなし・サイズ上限） | `reqwest` |
| `crates/coincheck/src/model.rs` | 応答の解析と検証（通貨コード・金額・エラーメッセージ） | `serde_json`, `rust_decimal` |
| `crates/coincheck/src/sign.rs` | HMAC-SHA256 署名（nonce + URL + 本文） | `hmac`, `sha2` |
| `crates/coincheck/src/credentials.rs` | 値を表示しない認証情報の型 | — |
| `crates/coincheck/src/error.rs` | 認証情報を含まないエラー型 | `thiserror` |
| `crates/gmocoin/src/client.rs` | `Client`：資産残高の取得（1 回の署名付き GET）、JPY 評価（残高の応答の円転レート）、価格の基準（販売所 / 取引所の最終価格） | `transport.rs`, `model.rs`, `sign.rs` |
| `crates/gmocoin/src/transport.rs` | `Transport` トレイトと HTTPS 実装（固定ホスト `api.coin.z.com`・リダイレクトなし・サイズ上限） | `reqwest` |
| `crates/gmocoin/src/model.rs` | 応答の解析と検証（銘柄・金額・`status` のエラー） | `serde_json`, `rust_decimal` |
| `crates/gmocoin/src/sign.rs` | HMAC-SHA256 署名（時刻 + メソッド + パス + 本文） | `hmac`, `sha2` |
| `crates/binance/src/client.rs` | `Client`：残高の取得（署名付き GET 1 回）と公開価格の取得、JPY 評価（JPY 市場、なければ USDT 市場 × USDTJPY）、価格の基準 | `transport.rs`, `model.rs`, `sign.rs` |
| `crates/binance/src/transport.rs` | `Transport` トレイトと HTTPS 実装（固定ホスト `api.binance.com`・リダイレクトなし・サイズ上限） | `reqwest` |
| `crates/binance/src/model.rs` | 応答の解析と検証（銘柄・金額・価格・`code` / `msg` のエラー） | `serde_json`, `rust_decimal` |
| `crates/binance/src/sign.rs` | HMAC-SHA256 署名（クエリ文字列） | `hmac`, `sha2` |
| `crates/binance/src/credentials.rs` | 値を表示しない認証情報の型 | — |
| `crates/binance/src/error.rs` | 認証情報を含まないエラー型 | `thiserror` |
| `crates/gmocoin/src/credentials.rs` | 値を表示しない認証情報の型 | — |
| `crates/gmocoin/src/error.rs` | 認証情報を含まないエラー型 | `thiserror` |
| `crates/portfolio/src/model.rs` | データ型（`Portfolio`、`Group`、`Category`、`Product`、`Account`、`Holding`、`Settings`）とスキーマのバージョン | `rust_decimal`, `serde` |
| `crates/portfolio/src/edit.rs` | `Edit` と `Portfolio::apply`（検証してから変更し、ID を採番する） | `validate.rs` |
| `crates/portfolio/src/validate.rs` | 入力の検査、`validate_integrity`（読み込み時の整合性）、`issues`（提案をブロックする不備） | `error.rs` |
| `crates/portfolio/src/error.rs` | `Error` と `Issue` | `thiserror` |
| `crates/suggest/src/engine.rs` | 評価額の換算、4 つのモード（1 口座または全口座）、口座への配り、結果の組み立て | `fill.rs`, `round.rs`, `portfolio` |
| `crates/suggest/src/overview.rs` | グループごと（目標はカテゴリの合計）、カテゴリごと、保有ごと、種別ごとの評価額、現在・目標の比率と差（グラフと一覧用。目標が未完成でも計算できる） | `engine.rs`, `portfolio` |
| `crates/suggest/src/account_balance.rs` | 口座内の商品バランス（商品比率に対する現在額と不足・超過） | `engine.rs`, `portfolio` |
| `crates/suggest/src/strategy.rs` | 商品と比率を指定した口座（戦略）の提案（4 つのモード）と、指定商品の比率に対する口座内バランス | `fill.rs`, `round.rs`, `engine.rs`, `portfolio` |
| `crates/suggest/src/fill.rs` | 水位配分（目標を最も下回るものから足す。売りは符号を反転して同じ関数を使う） | `rust_decimal` |
| `crates/suggest/src/round.rs` | 丸め単位への丸め（合計を保つ最大剰余法。上限つき） | `rust_decimal` |
| `crates/suggest/src/types.rs`, `error.rs` | `Mode`、`Request`、`Suggestion`、`Note`、`Error` | `portfolio` |
| `crates/browser/src/detect.rs` | Chrome・Edge・Brave のインストール先の検出（OS ごとの標準の場所） | — |
| `crates/browser/src/launch.rs` | 専用プロファイルでの起動と、`DevToolsActivePort` の読み取り。動作中のブラウザは再利用する | — |
| `crates/browser/src/cards.rs`, `crates/browser/js/read_cards.js` | 見出しの下のカードを行の組として読む固定の読み取り専用スクリプトと、返り値の検査・上限（Node のテストは `read_cards.test.mjs`） | — |
| `crates/browser/src/cdp.rs` | DevTools プロトコルの最小のクライアント（ページ一覧、接続、読み取り専用のスクリプト実行、終了） | `tungstenite` |
| `crates/tabular/src/decode.rs` | バイト列の文字コード判別（UTF-8 か Shift-JIS）。バイナリは拒否する | `encoding_rs` |
| `crates/tabular/src/parse.rs` | CSV・タブ区切りの解析（引用符、サイズの上限） | — |
| `crates/tabular/src/mapping.rs` | 名前・評価額の列の選択と、金額の読み取り。読めない行は理由つきで返す | `rust_decimal` |
| `crates/portfolio/src/presets.rs` | 組み込みのレシピ（プリセット）のデータ。口座を渡すレシピの定義（SBI の表タイトルと種別の規則 `types` を含む）と、レシピが指すプリセットの定義を返す `Recipe::effective`、表のタイトルから種別を引く `Recipe::type_of_title` | `portfolio` |
| `crates/portfolio/src/import.rs` | 取り込んだ行を別名で商品へ対応づける（`plan_import`。商品と種別の組ごと） | `portfolio` |
| `crates/export/src/lib.rs` | 保有・カテゴリ・商品・別名・口座・設定の CSV、全データの JSON、README の生成 | `portfolio` |
| `crates/export/src/csv.rs` | CSV の書き出し（BOM、CRLF、引用、数式の無効化） | — |
| `src-tauri/src/exporter.rs` | 新しいフォルダへの書き込み、UTC の日時の文字列 | `crates/export` |
| `crates/portfolio/src/valuation.rs` | 金額の JPY への換算（JPY と USD） | — |

## Docs

| ファイル | 役割 |
|---|---|
| `docs/specification.md` | 仕様の索引（状態・範囲・データフロー・制約）。詳細は次のファイル |
| `docs/specification-import.md` | 取得方法、表の取り込み、証券会社の出力、bitFlyer・Coincheck・GMO コイン・Binance の残高 |
| `docs/specification-browser.md` | ブラウザで取得（Browser Fetch） |
| `docs/specification-recipes.md` | 取得レシピ（リンクのたどり方、メニューのクリック、表の選び方） |
| `docs/specification-presets.md` | 組み込みのレシピ（プリセット）、ログインのページ、結果の反映 |
| `docs/specification-suggestions.md` | リバランスの計算、提案モード、口座タブ |
| `docs/specification-data.md` | ポートフォリオのモデル、口座、保存、エクスポート |
| `docs/ui-design.md` | UI 設計（レイアウトと各パネル） |
| `docs/ui-design-settings.md` | 設定タブのカード（bitFlyer、表の取り込み、ブラウザで取得、取得レシピ、保存先、Records、Groups） |
| `docs/design-constraints.md` | Design Constraints・Tauri Notes・CI コマンド・ライセンス・技術スタックの全文（`AI_CONTEXT.md` に要点） |
