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
- `docs/design-constraints.md`（Design Constraints などの全文）
- `docs/specification.md`（機能仕様の索引。詳細は `docs/specification-*.md`）
- `docs/ui-design.md`（UI 設計。設定タブは `docs/ui-design-settings.md`）

---

## Project Overview

**目的（個人開発）:** 日本の証券口座（SBI 証券・マネックス証券・bitFlyer）の資産を集約し、目標ポートフォリオに向けたリバランス案を提示する、外部依存のないクロスプラットフォームのデスクトップアプリ。

**技術スタックと主要ディレクトリ:** Tauri 2（Rust）+ TypeScript（vanilla）+ Vite。`src/`・`src-tauri/`（薄い境界層）・`crates/`（Tauri 非依存）・`docs/`・`scripts/`。詳細は [docs/design-constraints.md](docs/design-constraints.md)。

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

要点のみ。例外・条件を含む全文が正本: [docs/design-constraints.md](docs/design-constraints.md)（関連する変更の前に読む）。

- **外部ランタイム依存を持たない。** ブラウザ操作（任意の「ブラウザで取得」）は `crates/browser` に閉じ込め、行うのは表・カードの読み取りと同じサイトの通常リンクをたどることだけ。確認済みサイト（`portfolio::VETTED_SITES`）の別ホスト・メニューのクリックは、全文の条件を満たす例外のみ。入力・送信・ボタンやフォームの操作、注文・売買・入出金・ログアウトなどに見えるリンクのクリック（レシピに書かれていても）はしない。認証情報には触れない。
- **証券会社へのログインはアプリ内で行わない。** 利用者がシステムのブラウザで自分でログインする。証券会社ごとのコードは持たず、レシピ（データ）で読む。ブラウザ拡張は採らない。
- **ログイン情報（ID・パスワード・2FA コード）を保存しない。** bitFlyer と Coincheck の API キー・シークレットは OS のキーチェーンに保存し、ファイル・ログ・設定に平文で書かない。
- **データはローカルのみ。** 外部へ送信しない。テレメトリを入れない。
- **ロジックは Rust に寄せる。** TypeScript は表示と入力に限定する。まとまったロジックの切り出しは `docs/architecture.md` の Extraction Policy に従い、条件を満たす前に分けない。
- **金額計算は浮動小数点を使わない。** Rust 側で decimal 型を使う。
- **画面の自動読み取りは個人利用に限る。** 専用パーサーは作らず、列の対応づけは利用者が指定する。プリセットはデータとして出荷してよく、レシピは id への参照として保存する。名称は利用者が確認した別名で対応づけ、自動推測しない。
- **エクスポートには保有資産が含まれる。** 書き出し先はアプリが決めた新しいフォルダだけ。所有者のみの権限で作り、既存のものを上書きしない。
- **本アプリは投資助言ではない。** 売買は利用者が自分で行う。
- **旧コードを持ち込まない。** ポートフォリオの実データ（配分比率など）をリポジトリに含めない。

### Tauri Notes

コマンドは `src-tauri/src/lib.rs` に集約。金額・比率はフロントエンドでも十進数の文字列のまま扱う。`withGlobalTauri` は無効。CSP の許可先は最小限で追加し、`docs/architecture.md` に記録する。

### CI / Local Development Commands

`make all`（lint + test）を通す。CI は `security` → `lint` → `test` → `build`、`gate`（`CI`）が集約し、一覧は `DEVELOPING.md`。

### Pre-commit Hooks

`.pre-commit-config.yaml` で憲章のセキュリティ・品質チェックが走る。`main` への直接コミットは `check-not-on-default-branch` がブロックするため、`scripts/new-branch.sh` で作業ブランチを切る。

### docs/ Role

| ディレクトリ | 役割 | AI の編集 |
|---|---|---|
| `docs/` | 人間・AI 共用の詳細仕様書 | 更新可。ただし仕様・設計判断の変更は、ユーザーの依頼または提案への承認を経て記述する |
| `docs/dev-charter/` | 開発憲章（git subtree） | **禁止**（変更は dev-charter 本体に Issue を立て、`git subtree pull` で取り込む） |

### Licensing and Monetization

- コードは AGPL v3（`AGPL-3.0-only`）。外部コントリビューションは `CONTRIBUTING.md` の Contribution Terms（準 CLA）に同意した PR のみ受け入れる。同意のチェックがない PR はマージしない。
- 追加する依存は AGPL v3 と両立するライセンスか確認する。GPL 系などのコピーレフト依存は入れない。他者のコードは、ライセンスと再ライセンス可否を確認してから入れる。
- 独自の課金システムは作らない（GitHub Sponsors と Buy Me a Coffee のみ）。


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
