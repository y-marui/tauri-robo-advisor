# Architecture

## Overview

Tauri 2 デスクトップアプリ。ロジックはすべて Rust 側に置き、TypeScript のフロントエンドは表示と入力に限定する。
まとまったロジックは独立した Rust crate として切り出せる構造にし、必要になった時点で個別リポジトリへ分ける。

## Entry Points

- `src-tauri/src/main.rs` — デスクトップのエントリポイント（`lib.rs` の `run` を呼ぶだけ）
- `src-tauri/src/lib.rs` — Tauri のセットアップとコマンド登録（`propose_rebalance` は `rebalance` crate への薄い委譲）
- `crates/rebalance/src/lib.rs` — リバランス計算（`propose`）
- `crates/bitflyer/src/lib.rs` — bitFlyer クライアント（`Client`、`Credentials`、`Transport`）
- `crates/export/src/lib.rs` — `build(portfolio, info)`。各ファイルの内容をバイト列で返す
- `src-tauri/src/exporter.rs` — エクスポートのフォルダとファイルの書き込み（所有者のみの権限、上書きしない）
- `crates/suggest/src/lib.rs` — `suggest(portfolio, request)`、`Mode`、`Suggestion`
- `crates/portfolio/src/lib.rs` — `Portfolio`、`Edit`、`Portfolio::apply`、`validate_integrity`、`issues`
- `src-tauri/src/secrets.rs` — OS のキーチェーンへの API キーの保存（副作用はここに閉じ込める）
- `src-tauri/src/store.rs` — ポートフォリオの JSON ファイルの読み書き（アトミックな書き込み）
- `src-tauri/src/app_state.rs` — メモリ上のポートフォリオと保存先。コマンドから呼ばれ、Tauri なしでテストできる
- 保存（JSON ファイル）も同様に、ファイルの読み書きという副作用を `src-tauri` の 1 モジュールに閉じ込め、モデルと提案の計算は Tauri に依存しない crate に置く（[specification.md](specification.md) の Storage 参照）
- `src/main.ts` — フロントエンドのエントリポイント
- `index.html` — フロントエンドの HTML

## Directory Structure

| ディレクトリ | 役割 |
|---|---|
| `src/` | フロントエンド。表示と入力のみで、ドメインロジックを持たない |
| `src-tauri/src/` | Rust 側。Tauri のセットアップとコマンド（薄い境界層） |
| `crates/rebalance/` | リバランス計算。Tauri に依存しない純粋な Rust の crate（Cargo ワークスペースのメンバー） |
| `crates/bitflyer/` | bitFlyer の読み取り専用 API クライアント。署名・残高取得・JPY 評価。Tauri に依存しない |
| `crates/browser/` | Chrome・Edge・Brave の検出、専用プロファイルでの起動、DevTools プロトコル（localhost の WebSocket）での読み取り。ログインは利用者が手で行い、このクレートは認証情報に触れない。Tauri に依存しない |
| `crates/tabular/` | CSV・貼り付けテキストの読み取り（UTF-8 / Shift-JIS）と、名前・評価額の列の選択。証券会社ごとのコードは持たない。Tauri に依存しない |
| `crates/export/` | フルレコードのエクスポート（CSV・JSON・README の内容を作る）。Tauri に依存しない |
| `crates/suggest/` | 4 つの提案モードの計算（水位配分、丸め、口座の制約）。`portfolio` に依存し、Tauri には依存しない |
| `crates/portfolio/` | ポートフォリオのモデル（カテゴリ・商品・口座・保有・目標）と、検証つきの編集（`Edit`）。Tauri に依存しない |
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
| `thiserror` | `rebalance`・`bitflyer` のエラー型 |
| `reqwest`（`rustls`、default features なし） | bitFlyer への HTTPS 通信（`bitflyer`）。Tauri 経由で依存ツリーにも既にある |
| `hmac` / `sha2` | bitFlyer の API リクエスト署名（HMAC-SHA256） |
| `keyring` | OS のキーチェーンへの保存（macOS Keychain、Windows Credential Manager、Secret Service） |
| `serde_json`（`raw_value`） | bitFlyer の数値を丸めずに十進数へ読む |
| `vite` / `typescript` | フロントエンドのビルド |
