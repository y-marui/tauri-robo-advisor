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
| `src-tauri/src/lib.rs` | Tauri のセットアップ、`greet`（雛形）と `propose_rebalance` コマンド | `tauri`, `crates/rebalance` |
| `src-tauri/tauri.conf.json` | アプリ設定 | — |
| `src-tauri/capabilities/default.json` | 権限定義 | — |

## Domain

| ファイル | 役割 | 主な依存先 |
|---|---|---|
| `Cargo.toml` | Cargo ワークスペース定義と共通のパッケージ情報 | — |
| `crates/rebalance/src/lib.rs` | リバランス計算 `propose`、入力検証、テスト | `rust_decimal`, `serde`, `thiserror` |
