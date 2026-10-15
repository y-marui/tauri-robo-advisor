# AI_CONTEXT.md

> このファイルは Claude Code・GitHub Copilot など AI ツールが唯一参照するコンテキストファイルです。
> セッション開始時に必ず読み込んでください。

---

## Reference Order

AI はタスク開始時に以下の順で参照する:

1. `README.md`（概要・セットアップ）
2. `DEVELOPING.md`（ビルド・実装規約・命名規則）

必要に応じて以下を参照する（順不同）:

- `CONTRIBUTING.md`（PR・Issue ルール）
- `docs/architecture.md`（モジュール・コンポーネント構造）
- `docs/file-map.md`（ファイルレベルの依存関係 ※情報が足りない・古い場合は適宜探索し、追記・更新する）
- `docs/specification.md`（機能仕様・データフロー）
- `docs/ui-design.md`（UI 設計・コンポーネント仕様）

---

## Project Overview

**目的:** 日本の証券口座（SBI 証券・マネックス証券・bitFlyer）の資産を集約し、目標ポートフォリオに向けたリバランス案を提示する、外部依存のないクロスプラットフォームのデスクトップアプリ。

**チーム規模:** 個人開発。アジャイルで迅速な意思決定を重視。

**技術スタック:**

| 層 | 技術 |
|---|---|
| アプリ基盤 | Tauri 2（Rust 2024 edition） |
| フロントエンド | TypeScript（vanilla）+ Vite |
| パッケージ管理 | npm（`package-lock.json`）/ cargo（`Cargo.lock`） |
| Lint / Format | `tsc --noEmit` / `cargo fmt` / `cargo clippy -D warnings` |
| テスト | `npm test`（フロントエンドの単体テスト。Node 組み込みのテストランナー）/ `cargo test` |
| 対象 OS | macOS / Windows / Linux（デスクトップ優先。モバイルは対象外） |

**主要ディレクトリ:**

```
src/              # フロントエンド（TypeScript / CSS）
src-tauri/        # Rust 側（Tauri のセットアップとコマンド。薄い境界層）
crates/           # ドメインロジックの crate（Cargo ワークスペースのメンバー。Tauri 非依存）
docs/             # 仕様書・ナレッジベース（人間・AI 共用。仕様変更は依頼・承認を経て記述）
docs/dev-charter/ # 開発憲章（git subtree で取り込み。直接編集禁止）
scripts/          # dev-charter 配布の pre-commit 用チェックスクリプト
```

---

## Applied Charter Principles

憲章参照: `docs/dev-charter/CHARTER_INDEX.md` でトピックを特定してから該当ファイルのみ読む。

| 原則 | 要点 | 参照先 |
|---|---|---|
| 設計哲学 | 小さなツール・ローカルファースト・最小依存・オフライン優先 | `docs/dev-charter/topics/SOFTWARE_DESIGN_PRINCIPLES.md` |
| 変更設計 | 変更は必要最小限・YAGNI・既存パターン踏襲 | `docs/dev-charter/PRINCIPLES.md` |
| Git ワークフロー | Conventional Commits・`main` への直接 push 禁止・短命ブランチから PR | `docs/dev-charter/PROJECT_LIFECYCLE.md` |
| CI | `gate` job が必須チェック（`CI` / `Dev Charter`）・Draft PR は CI スキップ | `docs/dev-charter/topics/CI_POLICY.md` |
| セキュリティ | シークレット非コミット・pre-commit 二層構成 | `docs/dev-charter/SECURITY_POLICY.md` |
| 言語 | OSS のため公開面は英語、`README-jp.md` が日本語正本 | `docs/dev-charter/LANGUAGE_POLICY.md` |
| ライセンス | AGPL v3。将来の商用ライセンス提供のため準 CLA で運用する。著作権者名はフルネーム | `docs/dev-charter/LEGAL_POLICY.md`、`docs/dev-charter/topics/GITHUB_CONTRIBUTING.md` |
| UI | ライト/ダーク/システム対応・Unicode 絵文字禁止・Material Symbols 等を使用 | `docs/dev-charter/UI_GUIDELINES.md` |
| ローカライズ | 開発言語は英語、フォールバックも英語 | `docs/dev-charter/LOCALIZATION_POLICY.md` |
| TODO 管理 | TODO・ロードマップはリポジトリに書かず GitHub Issues で管理 | `docs/dev-charter/topics/GITHUB_PROJECT_MANAGEMENT.md` |

---

## Document Sync Rule

仕様・ルール・構成に変更が生じたとき、変更と同じ作業内で関連ドキュメントを更新する。
対象は docs/ 内のファイルに限らず、AI_CONTEXT.md・README.md 等のルートファイルも含む。

`README.md` と `README-jp.md` は日英ペアで、日本語版を正本として編集し、同一コミットで更新する。

---

## Project-Specific Rules

### Design Constraints

- **外部ランタイム依存を持たない。** Python・Selenium・別途インストールしたブラウザの操作を要求しない。
- **証券会社へのログインはアプリ内で行わない。** SBI 証券・マネックス証券のログインにはパスキーが必要で、Tauri の組み込み WebView では OS ごとに対応が不完全なため。利用者がシステムのブラウザで自分でログインし、保有一覧を CSV で保存するか、画面の表をコピーして、アプリに取り込む。出力できない証券会社は手入力にする。ブラウザ拡張は将来の選択肢で、必要になった時点で別途設計する。
- **ログイン情報（ID・パスワード・2FA コード）を保存しない。** bitFlyer の API キー・シークレットは OS のキーチェーンに保存し、ファイル・ログ・設定に平文で書かない。
- **データはローカルのみ。** 外部へ送信しない。テレメトリを入れない。
- **ロジックは Rust に寄せる。** TypeScript は表示と入力に限定し、計算・変換・検証を書かない。Tauri コマンドは薄い境界層にし、ドメインロジックは Tauri に依存しない純粋な Rust として書く。
- **まとまったロジックは個別の Rust リポジトリへの切り出しを検討する。** 基準と手順は `docs/architecture.md` の Extraction Policy に従う。条件を満たす前に分けない。切り出し先のリポジトリ名は `rust-` を先頭に付ける。
- **金額計算は浮動小数点を使わない。** Rust 側で decimal 型を使う。
- **証券会社の画面を自動で読み取る処理は個人利用に限る。** 将来ブラウザ拡張などで自動化する場合は、利用規約を確認し、画面構造の変更で壊れうることを前提に、取得部分を差し替え可能なモジュールに閉じ込める。取り込みは、列の対応づけを利用者が指定する汎用方式にし、証券会社ごとの専用パーサーは作らない（形式は会社ごと・版ごとに変わり、公開仕様もないため）。取得元の名称は、利用者が確認した別名で商品へ対応づけ、自動推測しない。
- **エクスポートには保有資産が含まれる。** 書き出し先はアプリが決めた新しいフォルダだけ（フロントエンドから指定させない）。所有者のみの権限で作り、既存のものを上書きしない。利用者が入力した名前は、表計算ソフトで数式として実行されないようテキストとして書く。
- **本アプリは投資助言ではない。** リバランス案は機械的な計算結果であり、売買は利用者が自分で行う。
- **過去のリポジトリ・旧アカウントとは無関係の新規実装。** 旧コードを持ち込まない。ポートフォリオの実データ（配分比率など）をリポジトリに含めない。

### Tauri Notes

- Rust 側のコマンドは `src-tauri/src/lib.rs` に集約し、フロントエンドからは `invoke` で呼ぶ。ポートフォリオは Rust が持ち、フロントエンドは `get_portfolio`（状態と不備の一覧）、`apply_portfolio_edit`（検証・保存は Rust）、`export_all`（全データの書き出し）を使う。`suggest_rebalance`（読み取り専用の提案）はコマンドとして公開済みだが、画面からはまだ呼んでいない（#15 で接続する）。
- 金額と比率は、フロントエンドでも十進数の文字列のまま扱い、数値型に変換しない。％と小数の変換は文字列の小数点の移動だけで行う（`src/decimal.ts`）。
- フロントエンドの単体テストは `src/*.test.ts`（`npm test`）。`@types/node` を入れないため、`tsc` の対象から外している。
- `src-tauri/tauri.conf.json` に CSP（本番 `csp` と開発用 `devCsp`）を設定してある。外部への接続や外部スクリプトが必要になったら、許可先を最小限で追加し、`docs/architecture.md` に理由を記録する。
- Cargo のルートは `Cargo.toml`（ワークスペース）。`Cargo.lock` はルートでコミットし、CI は `--locked` で実行する。
- `withGlobalTauri` は無効にしてある。フロントエンドは `@tauri-apps/api` を import して使い、`window.__TAURI__` に頼らない。

### CI / Local Development Commands

```sh
make install        # npm ci
make dev            # npm run tauri dev
make lint           # tsc --noEmit + cargo fmt --all --check + cargo clippy --workspace
make format         # cargo fmt
make test           # npm test + cargo test
make build          # npm run tauri build -- --no-bundle
make all            # lint + test
make update-charter # git subtree pull で dev-charter を最新化
```

CI（GitHub Actions）は push / PR のたびに `security` → `lint` → `test` → `build` の順で実行し、`gate`（`CI`）が結果を集約する。Linux ではシステムライブラリ（`libwebkit2gtk-4.1-dev` 等）が必要。

### Pre-commit Hooks

`.pre-commit-config.yaml` で憲章のセキュリティ・品質チェックが走る。`main` への直接コミットは `check-not-on-default-branch` がブロックするため、`scripts/new-branch.sh` で作業ブランチを切る。

### docs/ Role

| ディレクトリ | 役割 | AI の編集 |
|---|---|---|
| `docs/` | 人間・AI 共用の詳細仕様書 | 更新可。ただし仕様・設計判断の変更は、ユーザーの依頼または提案への承認を経て記述する |
| `docs/dev-charter/` | 開発憲章（git subtree） | **禁止**（変更は dev-charter 本体に Issue を立て、`git subtree pull` で取り込む） |

### Licensing

- コードは AGPL v3（SPDX: `AGPL-3.0-only`）。将来、商用ライセンスの提供と、自前サーバーアプリへの派生を想定している。
- 再ライセンスの余地を保つため、外部コントリビューションは `CONTRIBUTING.md` の Contribution Terms（準 CLA）に同意した PR のみ受け入れる。同意のチェックがない PR はマージしない。
- 追加する依存は AGPL v3 と両立するライセンスか確認する。商用ライセンスで配布できなくなるコピーレフト依存（GPL 系など）を入れない。
- 他者のコードを取り込む場合は、ライセンスと再ライセンス可否を確認してから入れる。

### Monetization

独自の課金システムは作らない。GitHub Sponsors と Buy Me a Coffee（`.github/FUNDING.yml`、README のバッジ）のみ。

---

## AI Tool Assignments

- **使用ツール**：Claude Code、Codex、GitHub Copilot、Gemini CLI
- **標準担当の正本**：`docs/dev-charter/AI_COLLABORATION_RULES.md` の「AI Tool Responsibilities」と「Rules for Multi-AI Usage」
- **プロジェクト固有の上書き**：なし

---

## Prohibited Actions

### Security Constraints

- シークレット・API キー・パスワード・トークンを**絶対にコードに書かない**
- `.env` ファイルをコミットしない（ダミー値のみの `.env.example` をコミットする）
- SSH 秘密鍵・クラウドトークンをコミットしない
- ローカル絶対パス（`/Users/...`、`/home/...`、`C:\Users\...`）をコードにハードコードしない
- 500 KB を超えるファイルをコミットしない
- AI との会話ログをリポジトリにコミットしない
- **シークレットを含むファイルやコードを AI に渡さない**（プロンプト・コンテキスト・スクリーンショット含む）
- **AI が生成したコードは必ずレビューしてからコミットする**
- 実際の口座情報・保有資産・配分比率をリポジトリ・Issue・テストデータに含めない

### Prohibited Out-of-Scope Changes

- **設計変更禁止**: ディレクトリ構造変更・モジュール移動（依頼がある場合を除く）
- **大規模リファクタ禁止**: 意図しない挙動変化を防ぐため（明示的に依頼された場合を除く）
- **依存追加禁止**: ライセンス・セキュリティリスクを人間がレビューするため。必要な場合は Issue を作成する
- **WIP コミット禁止**: 動作しないコードはコミットしない
- `docs/dev-charter/` 配下を AI が直接編集しない
