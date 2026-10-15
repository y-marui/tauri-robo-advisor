# File Map

_最終更新: 2026-10-15_

## Frontend

| ファイル | 役割 | 主な依存先 |
|---|---|---|
| `index.html` | HTML のエントリ | `src/main.ts`, `src/styles.css` |
| `src/main.ts` | アプリの起点。状態の保持、言語の切り替え、パネルの組み立て | `src/api.ts`, `src/views/*` |
| `src/api.ts` | Rust のコマンド（`get_portfolio`、`apply_portfolio_edit`、`get_overview`、`suggest_rebalance`）の呼び出し | `@tauri-apps/api/core` |
| `src/types.ts` | Rust の JSON に対応する型（金額は十進数の文字列） | — |
| `src/decimal.ts` | 十進数の文字列の変換（％ ↔ 小数、桁区切り表示）。浮動小数点を使わない | — |
| `src/i18n.ts` | 画面の文言（英語がキー、日本語は翻訳） | — |
| `src/panel.ts`, `src/fields.ts`, `src/dom.ts` | パネルの描画、入力部品、DOM の補助 | `src/i18n.ts` |
| `src/views/*.ts` | カテゴリ・商品・口座・保有・設定の各パネル、全体のバランス（グラフと提案）のパネル、不備の表示 | `src/panel.ts`, `src/fields.ts` |
| `src/*.test.ts` | Node の組み込みテストランナーによる単体テスト（`npm test`） | — |
| `src/styles.css` | スタイル | — |

## Backend

| ファイル | 役割 | 主な依存先 |
|---|---|---|
| `src-tauri/src/main.rs` | デスクトップのエントリ | `src-tauri/src/lib.rs` |
| `src-tauri/src/lib.rs` | Tauri のセットアップ、`propose_rebalance`、ポートフォリオ（`get_portfolio`、`apply_portfolio_edit`、`get_overview`、`suggest_rebalance`）と bitFlyer 関連のコマンド | `tauri`, `crates/rebalance`, `crates/bitflyer`, `src-tauri/src/secrets.rs` |
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
| `crates/suggest/src/overview.rs` | カテゴリごとの評価額と、現在・目標の比率（グラフ用。目標が未完成でも計算できる） | `engine.rs`, `portfolio` |
| `crates/suggest/src/fill.rs` | 水位配分（目標を最も下回るものから足す。売りは符号を反転して同じ関数を使う） | `rust_decimal` |
| `crates/suggest/src/round.rs` | 丸め単位への丸め（合計を保つ最大剰余法。上限つき） | `rust_decimal` |
| `crates/suggest/src/types.rs`, `error.rs` | `Mode`、`Request`、`Suggestion`、`Note`、`Error` | `portfolio` |
| `crates/export/src/lib.rs` | 保有・カテゴリ・商品・別名・口座・設定の CSV、全データの JSON、README の生成 | `portfolio` |
| `crates/export/src/csv.rs` | CSV の書き出し（BOM、CRLF、引用、数式の無効化） | — |
| `src-tauri/src/exporter.rs` | 新しいフォルダへの書き込み、UTC の日時の文字列 | `crates/export` |
| `crates/portfolio/src/valuation.rs` | 金額の JPY への換算（JPY と USD） | — |
