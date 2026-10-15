# File Map

_最終更新: 2026-10-15_

## Frontend

| ファイル | 役割 | 主な依存先 |
|---|---|---|
| `index.html` | HTML のエントリ | `src/main.ts`, `src/styles.css` |
| `src/main.ts` | アプリの起点。状態の保持、言語の切り替え、パネルの組み立て | `src/api.ts`, `src/views/*` |
| `src/api.ts` | Rust のコマンド（`get_portfolio`、`apply_portfolio_edit`、`get_overview`、`get_account_balance`、`suggest_rebalance`）の呼び出し | `@tauri-apps/api/core` |
| `src/types.ts` | Rust の JSON に対応する型（金額は十進数の文字列） | — |
| `src/decimal.ts` | 十進数の文字列の変換（％ ↔ 小数、桁区切り表示）。浮動小数点を使わない | — |
| `src/i18n.ts` | 画面の文言（英語がキー、日本語は翻訳） | — |
| `src/panel.ts`, `src/fields.ts`, `src/dom.ts` | パネルの描画、入力部品、DOM の補助 | `src/i18n.ts` |
| `src/views/holdings-table.ts` | 中央パネル。カテゴリ別の保有の一覧、小計、その場の編集 | `src/panel.ts`, `src/fields.ts` |
| `src/views/account-tabs.ts` | 右パネル。口座ごとのタブ、口座内の商品バランス、口座への追加の提案 | `src/views/suggestion.ts` |
| `src/views/suggestion.ts` | 提案の入力欄と結果の表示（全体のバランスと口座のタブで共有） | `src/api.ts` |
| `src/views/bitflyer.ts` | bitFlyer のカード。キーの保存・削除、残高の取得と対応づけ、保有の置き換え | `src/api.ts` |
| `src/views/browser-fetch.ts` | 「ブラウザで取得」。ブラウザの選択・起動、ページと表の選択、閉じる | `src/api.ts` |
| `src-tauri/src/browser_session.rs` | 専用ブラウザの接続先と、取得した表の保持。ページはホストとタイトルだけを返す。終了時に閉じる | `crates/browser`, `crates/tabular` |
| `src/views/table-import.ts` | 表の取り込みのカード（ファイル・貼り付け・列の指定・保存） | `src/views/import-plan.ts` |
| `src/views/import-plan.ts` | 取り込み結果の表示（対応づけ・別名の追加と解除・保有の置き換え）。bitFlyer と表の取り込みで共有 | `src/api.ts` |
| `src/views/*.ts` | カテゴリ・商品・口座・保有・設定の各パネル、全体のバランス（グラフと提案）のパネル、不備の表示 | `src/panel.ts`, `src/fields.ts` |
| `src/*.test.ts` | Node の組み込みテストランナーによる単体テスト（`npm test`） | — |
| `src/styles.css` | スタイル | — |

## Backend

| ファイル | 役割 | 主な依存先 |
|---|---|---|
| `src-tauri/src/main.rs` | デスクトップのエントリ | `src-tauri/src/lib.rs` |
| `src-tauri/src/lib.rs` | Tauri のセットアップ、`propose_rebalance`、ポートフォリオ（`get_portfolio`、`apply_portfolio_edit`、`get_overview`、`get_account_balance`、`suggest_rebalance`）と bitFlyer 関連のコマンド | `tauri`, `crates/rebalance`, `crates/bitflyer`, `src-tauri/src/secrets.rs` |
| `src-tauri/src/store.rs` | ポートフォリオ JSON の読み書き（一時ファイル → 同期 → 置き換え、権限 0600、サイズ上限） | `crates/portfolio` |
| `src-tauri/src/app_state.rs` | メモリ上のポートフォリオ。編集を検証・保存してから反映する | `store.rs`, `crates/portfolio` |
| `src-tauri/src/secrets.rs` | OS キーチェーンへの API キーの保存・読み出し・削除と入力検証 | `keyring`, `crates/bitflyer` |
| `src-tauri/tauri.conf.json` | アプリ設定 | — |
| `src-tauri/capabilities/default.json` | 権限定義 | — |

## Domain

| ファイル | 役割 | 主な依存先 |
|---|---|---|
| `Cargo.toml` | Cargo ワークスペース定義と共通のパッケージ情報 | — |
| `crates/rebalance/src/lib.rs` | リバランス計算 `propose`、入力検証、テスト | `rust_decimal`, `serde`, `thiserror` |
| `crates/bitflyer/src/client.rs` | `Client`：残高取得、JPY 評価、署名付きリクエスト生成 | `transport.rs`, `model.rs`, `sign.rs`, `rebalance` |
| `crates/bitflyer/src/transport.rs` | `Transport` トレイトと HTTPS 実装（リダイレクトなし・サイズ上限） | `reqwest` |
| `crates/bitflyer/src/model.rs` | 応答の解析と検証（通貨コード・金額・エラーメッセージ） | `serde_json`, `rust_decimal` |
| `crates/bitflyer/src/sign.rs` | HMAC-SHA256 署名 | `hmac`, `sha2` |
| `crates/bitflyer/src/credentials.rs` | 値を表示しない認証情報の型 | — |
| `crates/bitflyer/src/error.rs` | 認証情報を含まないエラー型 | `thiserror` |
| `crates/portfolio/src/model.rs` | データ型（`Portfolio`、`Category`、`Product`、`Account`、`Holding`、`Settings`）とスキーマのバージョン | `rust_decimal`, `serde` |
| `crates/portfolio/src/edit.rs` | `Edit` と `Portfolio::apply`（検証してから変更し、ID を採番する） | `validate.rs` |
| `crates/portfolio/src/validate.rs` | 入力の検査、`validate_integrity`（読み込み時の整合性）、`issues`（提案をブロックする不備） | `error.rs` |
| `crates/portfolio/src/error.rs` | `Error` と `Issue` | `thiserror` |
| `crates/suggest/src/engine.rs` | 評価額の換算、4 つのモード、結果の組み立て | `fill.rs`, `round.rs`, `portfolio` |
| `crates/suggest/src/overview.rs` | カテゴリごとと保有ごとの評価額、現在・目標の比率と差（グラフと一覧用。目標が未完成でも計算できる） | `engine.rs`, `portfolio` |
| `crates/suggest/src/account_balance.rs` | 口座内の商品バランス（商品比率に対する現在額と不足・超過） | `engine.rs`, `portfolio` |
| `crates/suggest/src/fill.rs` | 水位配分（目標を最も下回るものから足す。売りは符号を反転して同じ関数を使う） | `rust_decimal` |
| `crates/suggest/src/round.rs` | 丸め単位への丸め（合計を保つ最大剰余法。上限つき） | `rust_decimal` |
| `crates/suggest/src/types.rs`, `error.rs` | `Mode`、`Request`、`Suggestion`、`Note`、`Error` | `portfolio` |
| `crates/browser/src/detect.rs` | Chrome・Edge・Brave のインストール先の検出（OS ごとの標準の場所） | — |
| `crates/browser/src/launch.rs` | 専用プロファイルでの起動と、`DevToolsActivePort` の読み取り。動作中のブラウザは再利用する | — |
| `crates/browser/src/cdp.rs` | DevTools プロトコルの最小のクライアント（ページ一覧、接続、読み取り専用のスクリプト実行、終了） | `tungstenite` |
| `crates/tabular/src/decode.rs` | バイト列の文字コード判別（UTF-8 か Shift-JIS）。バイナリは拒否する | `encoding_rs` |
| `crates/tabular/src/parse.rs` | CSV・タブ区切りの解析（引用符、サイズの上限） | — |
| `crates/tabular/src/mapping.rs` | 名前・評価額の列の選択と、金額の読み取り。読めない行は理由つきで返す | `rust_decimal` |
| `crates/portfolio/src/import.rs` | 取り込んだ行を別名で商品へ対応づける（`plan_import`） | `portfolio` |
| `crates/export/src/lib.rs` | 保有・カテゴリ・商品・別名・口座・設定の CSV、全データの JSON、README の生成 | `portfolio` |
| `crates/export/src/csv.rs` | CSV の書き出し（BOM、CRLF、引用、数式の無効化） | — |
| `src-tauri/src/exporter.rs` | 新しいフォルダへの書き込み、UTC の日時の文字列 | `crates/export` |
| `crates/portfolio/src/valuation.rs` | 金額の JPY への換算（JPY と USD） | — |
