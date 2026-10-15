# File Map

_最終更新: 2026-10-15_

## Frontend

| ファイル | 役割 | 主な依存先 |
|---|---|---|
| `index.html` | HTML のエントリ | `src/main.ts`, `src/styles.css` |
| `src/main.ts` | 雛形のデモ（`greet` コマンドの呼び出し） | `@tauri-apps/api/core` |
| `src/styles.css` | スタイル | — |

## Backend

| ファイル | 役割 | 主な依存先 |
|---|---|---|
| `src-tauri/src/main.rs` | デスクトップのエントリ | `src-tauri/src/lib.rs` |
| `src-tauri/src/lib.rs` | Tauri のセットアップ、`greet`（雛形）、`propose_rebalance`、bitFlyer 関連のコマンド | `tauri`, `crates/rebalance`, `crates/bitflyer`, `src-tauri/src/secrets.rs` |
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
