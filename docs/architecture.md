# Architecture

## Overview

Tauri 2 デスクトップアプリ。ロジックはすべて Rust 側に置き、TypeScript のフロントエンドは表示と入力に限定する。
まとまったロジックは独立した Rust crate として切り出せる構造にし、必要になった時点で個別リポジトリへ分ける。

## Entry Points

- `src-tauri/src/main.rs` — デスクトップのエントリポイント（`lib.rs` の `run` を呼ぶだけ）
- `src-tauri/src/lib.rs` — Tauri のセットアップとコマンド登録（`propose_rebalance` は `rebalance` crate への薄い委譲）
- `crates/rebalance/src/lib.rs` — リバランス計算（`propose`）
- `src/main.ts` — フロントエンドのエントリポイント
- `index.html` — フロントエンドの HTML

## Directory Structure

| ディレクトリ | 役割 |
|---|---|
| `src/` | フロントエンド。表示と入力のみで、ドメインロジックを持たない |
| `src-tauri/src/` | Rust 側。Tauri のセットアップとコマンド（薄い境界層） |
| `crates/rebalance/` | リバランス計算。Tauri に依存しない純粋な Rust の crate（Cargo ワークスペースのメンバー） |
| `src-tauri/capabilities/` | Tauri の権限定義 |
| `docs/` | 仕様書・ナレッジベース |

## Layering Policy

```
Frontend (TypeScript) → Tauri command (thin) → Domain logic (Rust) → Storage / Broker data (file import, API)
```

- フロントエンドは `invoke` で Rust のコマンドを呼ぶだけにする。計算・変換・検証はフロントエンドに書かない。
- Tauri コマンドは引数の受け渡しと結果の返却だけを行い、ドメインロジックを直接書かない。
- ドメインロジックは Tauri に依存しない純粋な Rust として書き、`cargo test` で単体テストできるようにする。
- 逆依存禁止。循環依存禁止。

## Workspace

ルートの `Cargo.toml` は Cargo ワークスペースで、メンバーは `src-tauri` と `crates/*`。`Cargo.lock` と `target/` はルートに置く。
ドメインロジックは `crates/` の crate に置き、Tauri への依存を持たせない。コンパイル境界で Tauri 非依存を保証でき、`cargo test -p <crate>` で単独にテストできる。

## Extraction Policy

まとまったロジックは、次のいずれかを満たした時点で個別の Rust リポジトリ（crate）への切り出しを検討する。条件を満たす前に分けない（YAGNI）。

- Tauri に依存せず単独で意味を持ち、アプリ以外（CLI や別アプリ）からも使いたい。
- 公開 API が安定し、独立してバージョン管理・リリースしたい。
- 変更頻度や担当がアプリ本体と明確に異なる。

切り出すときの手順:

1. まず同一リポジトリ内で Cargo workspace のメンバー crate（`crates/`）として分離し、境界を確かめる。`rebalance` はこの段階にある。
2. 境界が安定したら個別リポジトリへ移す。リポジトリ名は `rust-` を先頭に付ける（例: `rust-rebalance`）。
3. アプリ側は crate の依存として参照する。バージョンはタグで固定する。
4. 切り出しの判断と経緯はこのファイルに記録し、作業の進捗は Issue で管理する。

実装予定の作業や切り出しの候補は、このファイルではなく GitHub Issues で管理する。

## Key Dependencies

| ライブラリ / モジュール | 用途 |
|---|---|
| `tauri` | アプリ基盤 |
| `serde` / `serde_json` | シリアライズ |
| `rust_decimal` | 金額の正確な十進演算（`rebalance`）。IPC では文字列として受け渡す |
| `thiserror` | `rebalance` のエラー型 |
| `vite` / `typescript` | フロントエンドのビルド |
