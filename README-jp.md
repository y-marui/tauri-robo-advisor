# tauri-robo-advisor

> **このファイルは正本（日本語版）です。**
> 英語版（参照）は [README.md](README.md) を参照してください。

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![CI](https://github.com/y-marui/tauri-robo-advisor/actions/workflows/ci.yml/badge.svg)](https://github.com/y-marui/tauri-robo-advisor/actions/workflows/ci.yml)
[![Charter Check](https://github.com/y-marui/tauri-robo-advisor/actions/workflows/dev-charter-check.yml/badge.svg)](https://github.com/y-marui/tauri-robo-advisor/actions/workflows/dev-charter-check.yml)
[![GitHub Sponsors](https://img.shields.io/github/sponsors/y-marui?style=social)](https://github.com/sponsors/y-marui)
[![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-donate-yellow.svg)](https://www.buymeacoffee.com/y.marui)

日本の証券口座の資産を集約し、自分で決めた目標ポートフォリオに向けたリバランス案を提示する、外部ランタイム依存のないクロスプラットフォームのデスクトップアプリです。[Tauri 2](https://tauri.app/) で開発しています。

> 状態: 初期の雛形です。機能はまだ実装されていません。

## Requirements

- Node.js 20 以上
- Rust (stable) と、お使いの OS 向けの [Tauri の前提条件](https://tauri.app/start/prerequisites/)
- 対応 OS: macOS、Windows、Linux（デスクトップのみ）

## Setup

~~~sh
make install   # npm ci
make dev       # 開発モードでアプリを起動
make build     # アプリをビルド（インストーラのバンドルなし）
~~~

## Project Structure

~~~text
src/         # フロントエンド（TypeScript、CSS）: 表示と入力のみ
src-tauri/   # Rust: Tauri のセットアップと薄いコマンド層
crates/      # Rust: ドメインロジックの crate（Tauri 非依存）
docs/        # 仕様書・ナレッジベース
scripts/     # dev-charter 配布のチェックスクリプト
~~~

## Usage

現在は雛形のため、操作できる機能はありません。開発用コマンドは次のとおりです。

| コマンド | 説明 |
|---|---|
| `make dev` | 開発モードでアプリを起動 |
| `make lint` | 型検査、`cargo fmt --check`、`cargo clippy` |
| `make test` | フロントエンドと Rust のテストを実行 |
| `make build` | バンドルなしでアプリをビルド |
| `make all` | lint と test |

## Supported Sources

組み込みのプリセットは、ログイン済みのブラウザのページにある保有の表を読みます（ログインは利用者が自分で行います）。「確認済み」は、持ち主が実際のページの構造を確かめたという意味で、保証ではありません（ページが変わるとプリセットが読めなくなることがあります）。

| 取得元 | 読むもの | 状態 |
|---|---|---|
| SBI 証券 | 国内株式、投資信託 | 確認済み |
| マネックス証券 | 投資信託 | 確認済み |
| 楽天証券 | 円建ての投資信託 | 確認済み。国内株式は種別の列で読み分ける（#88）が、作り物のページのテストだけで確かめてあり、種別の文字は実際のページで未確認 |
| 大和コネクト証券 | 国内株式、現金（円） | 保有銘柄一覧は確認済み。現金のページは確認済みの記録なし |
| bitFlyer | REST API の残高（読み取り専用のキーを OS のキーチェーンに保存） | 実際のサービスで読み取り専用のキーで確認済み（認証と残高）。bitFlyer Lightning に円の市場がない通貨は除外して知らせる。価格は公開の価格ページから取ることもできる（表はブラウザ経由で読み、Lightning の価格を比較に出す） |
| PayPay 証券 | 保有銘柄のカード（評価額だけ。数量・価格は出ない）。暫定のプリセット | 構造は実際のページで確認済み（2 件）。「もっと見る」・多数の保有は未確認（#76） |

ほかの取得元は、CSV や貼り付けた表を自分で列を指定して取り込むか、手入力します。

## Design Principles

- 外部ランタイムに依存しない（Python、Selenium、別途インストールしたブラウザを必要としない）。
- ローカル完結。データは端末の外へ出さない。ログイン情報は保存せず、API キーは OS のキーチェーンに保存する。
- デスクトップを優先し、モバイルは対象外とする。

## Documentation

- [DEVELOPING.md](DEVELOPING.md): ビルド・テスト・実装規約
- [CONTRIBUTING.md](CONTRIBUTING.md): PR・Issue のルール
- [docs/architecture.md](docs/architecture.md): モジュール・コンポーネント構造
- [docs/specification.md](docs/specification.md): 仕様とデータフロー（索引。詳細は `docs/specification-*.md`）
- [docs/ui-design.md](docs/ui-design.md): UI 設計（設定タブは `docs/ui-design-settings.md`）
- [docs/design-constraints.md](docs/design-constraints.md): 設計上の制約とプロジェクトの規則の全文

## Disclaimer

個人利用を想定しています。投資助言ではなく、リバランス案は機械的な計算結果です。利用前に各証券会社の利用規約を確認してください。

## Contributing

[CONTRIBUTING.md](CONTRIBUTING.md) を参照してください。

## License

[AGPL v3](LICENSE)。将来、商用ライセンスを提供する可能性があります。コントリビューションは [CONTRIBUTING.md](CONTRIBUTING.md) の条件で受け付けます。

---

*この文書には英語版 [README.md](README.md) があります。編集時は同一コミットで更新してください。*
