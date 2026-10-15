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
| `make test` | Rust のテストを実行 |
| `make build` | バンドルなしでアプリをビルド |
| `make all` | lint と test |

## Design Principles

- 外部ランタイムに依存しない（Python、Selenium、別途インストールしたブラウザを必要としない）。
- ローカル完結。データは端末の外へ出さない。ログイン情報は保存せず、API キーは OS のキーチェーンに保存する。
- デスクトップを優先し、モバイルは対象外とする。

## Documentation

- [DEVELOPING.md](DEVELOPING.md): ビルド・テスト・実装規約
- [CONTRIBUTING.md](CONTRIBUTING.md): PR・Issue のルール
- [docs/architecture.md](docs/architecture.md): モジュール・コンポーネント構造
- [docs/specification.md](docs/specification.md): 仕様とデータフロー
- [docs/ui-design.md](docs/ui-design.md): UI 設計

## Disclaimer

個人利用を想定しています。投資助言ではなく、リバランス案は機械的な計算結果です。利用前に各証券会社の利用規約を確認してください。

## Contributing

[CONTRIBUTING.md](CONTRIBUTING.md) を参照してください。

## License

[AGPL v3](LICENSE)。将来、商用ライセンスを提供する可能性があります。コントリビューションは [CONTRIBUTING.md](CONTRIBUTING.md) の条件で受け付けます。

---

*この文書には英語版 [README.md](README.md) があります。編集時は同一コミットで更新してください。*
