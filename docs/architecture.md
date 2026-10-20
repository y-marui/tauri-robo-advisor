# Architecture

## Overview

Tauri 2 デスクトップアプリ。ロジックはすべて Rust 側に置き、TypeScript のフロントエンドは表示と入力に限定する。
まとまったロジックは独立した Rust crate として切り出せる構造にし、必要になった時点で個別リポジトリへ分ける。

## Entry Points

- `src-tauri/src/main.rs` — デスクトップのエントリポイント（`lib.rs` の `run` を呼ぶだけ）
- `src-tauri/src/lib.rs` — Tauri のセットアップとコマンド登録（`propose_rebalance` は `rebalance` crate への薄い委譲）
- `crates/rebalance/src/lib.rs` — リバランス計算（`propose`）
- `crates/bitflyer/src/lib.rs` — bitFlyer クライアント（`Client`、`Credentials`、`Transport`、価格ページの表を読む `PagePrices`）
- `crates/coincheck/src/lib.rs` — Coincheck クライアント（`Client`、`Credentials`、`Transport`。公開のティッカーで評価）
- `crates/gmocoin/src/lib.rs` — GMO コインクライアント（`Client`、`Credentials`、`Transport`、`PriceBasis`。残高の応答の円転レート（販売所の売値）で評価）
- `crates/export/src/lib.rs` — `build(portfolio, info)`。各ファイルの内容をバイト列で返す
- `src-tauri/src/exporter.rs` — エクスポートのフォルダとファイルの書き込み（所有者のみの権限、上書きしない）
- `crates/suggest/src/lib.rs` — `suggest(portfolio, request)`、`Mode`、`Suggestion`
- `crates/portfolio/src/lib.rs` — `Portfolio`、`Edit`、`Portfolio::apply`、`validate_integrity`、`issues`
- `src-tauri/src/secrets.rs` — OS のキーチェーンへの API キーの保存（副作用はここに閉じ込める）
- `src-tauri/src/store.rs` — ポートフォリオの JSON ファイルの読み書き（アトミックな書き込み）
- `src-tauri/src/app_state.rs` — メモリ上のポートフォリオと保存先。コマンドから呼ばれ、Tauri なしでテストできる
- 保存（JSON ファイル）も同様に、ファイルの読み書きという副作用を `src-tauri` の 1 モジュールに閉じ込め、モデルと提案の計算は Tauri に依存しない crate に置く（[specification-data.md](specification-data.md) の Storage 参照）
- `src/main.ts` — フロントエンドのエントリポイント
- `index.html` — フロントエンドの HTML

## Directory Structure

| ディレクトリ | 役割 |
|---|---|
| `src/` | フロントエンド。表示と入力のみで、ドメインロジックを持たない |
| `src-tauri/src/` | Rust 側。Tauri のセットアップとコマンド（薄い境界層） |
| `crates/rebalance/` | リバランス計算。Tauri に依存しない純粋な Rust の crate（Cargo ワークスペースのメンバー） |
| `crates/bitflyer/` | bitFlyer の読み取り専用 API クライアント。署名・残高取得・JPY 評価。Tauri に依存しない |
| `crates/coincheck/` | Coincheck の読み取り専用 API クライアント。署名・残高取得・JPY 評価。Tauri に依存しない。`Transport`・`Holding` などは `crates/bitflyer` と同形の重複。共通 crate への抽出は見送り（Extraction Policy の Decision を参照） |
| `crates/gmocoin/` | GMO コインの読み取り専用 API クライアント。署名・残高取得・JPY 評価（残高の応答の `conversionRate`、価格の基準を保有ごとに持つ）。Tauri に依存しない。共通部分の抽出は見送り（下記 Extraction Policy の判断を参照） |
| `crates/browser/` | Chrome・Edge・Brave の検出、専用プロファイルでの起動、DevTools プロトコル（localhost の WebSocket）での読み取り。ログインは利用者が手で行い、このクレートは認証情報に触れない。Tauri に依存しない。読む表は、ほかの表を含まない表と、中の表がどれも深い入れ子を持たない外側の表の 2 つの組（`PageTables`）で返し、どちらを使うかは書き出しに合うかで決まる（葉の表を先に探す。ほかの入れ子の表は、ページの配置用なので読まない）。表のないページ向けに、見出しの下のカード（同じ形のブロックの繰り返し）も、固定の読み取り専用スクリプト（`js/read_cards.js`）で行の組にして返し、以降は表と同じ流れに載せる（#76。仕様は `docs/specification.md`） |
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

### Decision: API crates (bitflyer, coincheck, gmocoin)

3 つ目の API crate（`gmocoin`）を作った時点で、`Transport`・`Request`・`Response`・`Credentials`・`Holding` の共通 crate への抽出を検討し、見送った（#126）。

- 上の 3 つの条件のどれも満たさない。3 つの crate はアプリ内でしか使わず、API は安定しておらず、変更の担当と頻度もアプリ本体と同じ。
- 重複は、`transport.rs`（固定のホストと 20 行ほどの定数）と `credentials.rs`（約 30 行）が中心で、署名（bitFlyer は時刻+メソッド+パス、Coincheck は nonce+URL、GMO コインは時刻+メソッド+パス+本文）、応答の解析、エラーの文言、`Holding`（GMO コインだけが価格の基準を持つ）は取引所ごとに違う。共通部分を切り出すと、固定のホストをパラメーターにする必要があり、「ホストは crate が固定する」という安全上の性質が弱まる。
- 既存の 2 つの crate（読み取り済み・実サービスで確認済みの bitFlyer を含む）を触るリスクに比べ、減る重複の量が小さい。4 つ目（Binance）が決まり、同じ形の変更が複数の crate に必要になった時点で、再度判断する。

実装予定の作業や切り出しの候補は、このファイルではなく GitHub Issues で管理する。

### Reloading the data file (#43)

- 再読み込みは、ファイルの更新時刻とサイズ（フィンガープリント）が変わったときだけ読む。同期サービスが、更新時刻とサイズを変えずに中身だけ置き換えると、次の編集まで画面は古いままになる（保存は内容のハッシュで守られ、上書きは断られる）。
- ハッシュ（最大 10 MiB のファイル全体の読み込み）を再読み込みのたびに比べる案は、今は採らない。再読み込みは、ほぼすべてのコマンド（概況の取得なども）の前に走り、同期フォルダではファイルの読み込みが遅い（ダウンロードの待ち）こともあるため。実際に起きたら、ウィンドウに戻ったときの `get_portfolio` だけで比べる形を検討する。
- フォルダの切り替えとの競合は、データの場所の番号（`Disk::location`）で防ぐ。`apply_portfolio_edit` は番号を受け取り、`AppState::apply_at` が編集と同じロックの中で比べる。

## Key Dependencies

| ライブラリ / モジュール | 用途 |
|---|---|
| `tauri` | アプリ基盤 |
| `serde` / `serde_json` | シリアライズ |
| `rust_decimal` | 金額の正確な十進演算（`rebalance`）。IPC では文字列として受け渡す |
| `thiserror` | `rebalance`・`bitflyer`・`coincheck`・`gmocoin` のエラー型 |
| `reqwest`（`rustls`、default features なし） | bitFlyer・Coincheck・GMO コインへの HTTPS 通信（`bitflyer`・`coincheck`・`gmocoin`）。Tauri 経由で依存ツリーにも既にある |
| `hmac` / `sha2` | bitFlyer・Coincheck・GMO コインの API リクエスト署名（HMAC-SHA256） |
| `keyring` | OS のキーチェーンへの保存（macOS Keychain、Windows Credential Manager、Secret Service） |
| `serde_json`（`raw_value`） | bitFlyer・Coincheck・GMO コインの数値を丸めずに十進数へ読む |
| `vite` / `typescript` | フロントエンドのビルド |
