# Design Constraints

[AI_CONTEXT.md](../AI_CONTEXT.md) の「Project-Specific Rules」のうち、「Design Constraints」「Tauri Notes」「CI / Local Development Commands」「Licensing」「Monetization」の全文。AI_CONTEXT.md には要点だけを置き、詳細な規則はここを正本とする。これらの規則は、すべて守る。

## Design Constraints

- **外部ランタイム依存を持たない。** Python・Selenium・chromedriver を要求しない。利用者が自分で入れたシステムのブラウザ（Chrome・Edge・Brave）を任意で使う「ブラウザで取得」は設けてよいが、必須にしない（ブラウザがなくてもアプリのすべてが動く）。ブラウザの操作は `crates/browser` に閉じ込める。行うのは、表の読み取りと、見出しの下で繰り返されるブロック（カード）の文字の読み取り（#76。どちらも読むだけ）と、別のページを表示するだけの通常のリンク（行き先のアドレスを持つ `<a>`。スクリプトで動くリンクは除く）をたどることだけ（#36）。たどれるのは、今のページと同じサイトのリンクに限る。**例外として、レシピ（プリセットのデータ）に確認済みのサイト（`portfolio::VETTED_SITES`。会社が持つ登録ドメインだけで、多くの持ち主が共有するドメインは入れない）を明示したときは、レシピを始めたページのホストを含むそのサイトの別のホストへのリンクもたどれる**（`https`、ポートなし、行き先は承認したものと完全に一致。着いたページのオリジンも確かめる）。**もう一つの例外として、確認済みのサイトに限り、ページ自身のスクリプトで動くメニューの項目（`javascript:` や `#` の `<a>`）を、メニューの組の見出しの文字と項目の文字がともに完全に一致するただ 1 つのものとして、クリックしてよい**（レシピがメニューの組と、クリック後に見えるべき見出しを持ち、見出しが違えば止める。戻る操作はしない。大和コネクト証券のようにフォームの POST とスクリプトのリンクで動くサイト用）。入力・送信・ボタンやフォームの操作はしない。注文・売買・入出金・ログアウトなどに見えるリンクは、レシピに書かれていても押さない。認証情報には触れない。
- **証券会社へのログインはアプリ内で行わない。** SBI 証券・マネックス証券のログインにはパスキーが必要で、Tauri の組み込み WebView では OS ごとに対応が不完全なため。利用者がシステムのブラウザで自分でログインし、保有一覧を CSV で保存するか、画面の表をコピーして、アプリに取り込む。出力できない証券会社は手入力にする。ログイン後の表を自動で読むには、アプリが専用のプロファイルでブラウザを起動し、利用者がそこで自分でログインしたあと、アプリが DevTools プロトコル（localhost のみ）で表を読む（#28）。ブラウザ拡張は採らない。ログイン後は、利用者が保存した「レシピ」（たどるリンクの文字、表の書き出し、列の指定、口座）で、リンクをたどって表を読む。証券会社ごとのコードは持たず、ページが変わったときはレシピを直す。
- **ログイン情報（ID・パスワード・2FA コード）を保存しない。** bitFlyer の API キー・シークレットは OS のキーチェーンに保存し、ファイル・ログ・設定に平文で書かない。
- **データはローカルのみ。** 外部へ送信しない。テレメトリを入れない。保存先に OS が同期するフォルダ（iCloud Drive など）を利用者が選ぶのは許す（同期は OS の仕事で、アプリは何も送らない。古い版で上書きしない保護を持つ。#35）。
- **ロジックは Rust に寄せる。** TypeScript は表示と入力に限定し、計算・変換・検証を書かない。Tauri コマンドは薄い境界層にし、ドメインロジックは Tauri に依存しない純粋な Rust として書く。
- **まとまったロジックは個別の Rust リポジトリへの切り出しを検討する。** 基準と手順は `docs/architecture.md` の Extraction Policy に従う。条件を満たす前に分けない。切り出し先のリポジトリ名は `rust-` を先頭に付ける。
- **金額計算は浮動小数点を使わない。** Rust 側で decimal 型を使う。
- **証券会社の画面を自動で読み取る処理は個人利用に限る。** 将来ブラウザ拡張などで自動化する場合は、利用規約を確認し、画面構造の変更で壊れうることを前提に、取得部分を差し替え可能なモジュールに閉じ込める。取り込みは、列の対応づけを利用者が指定する汎用方式にし、証券会社ごとの専用パーサーは作らない（形式は会社ごと・版ごとに変わり、公開仕様もないため）。ただし、実際のページの構造（見出しと形だけ）を確認して作った**組み込みのレシピ（プリセット）をデータとして出荷してよい**。プリセットは利用者が書くレシピと同じ形で、コードではなく `crates/portfolio/src/presets.rs` のデータとして持ち、画面の変更はデータの修正で直す。口座は利用者が選ぶ。プリセットから作ったレシピは、**コピーではなくプリセットの id への参照**として保存し（口座と、覚えたリンクの行き先だけを持つ）、定義はアプリが持つ最新のプリセットから毎回読む（アプリの更新がプリセットの更新になる）。一覧では読み取り専用で、削除と「自分のレシピにする」（その時点の定義で普通のレシピにし、以後は追従しない）ができる。存在しないプリセットの id は、そのレシピだけのエラーにする（データは残す）。検証（確認済みのサイト、メニューのクリックの規則）は、定義した結果に対して行う。取得元の名称は、利用者が確認した別名で商品へ対応づけ、自動推測しない。
- **エクスポートには保有資産が含まれる。** 書き出し先はアプリが決めた新しいフォルダだけ（フロントエンドから指定させない）。所有者のみの権限で作り、既存のものを上書きしない。利用者が入力した名前は、表計算ソフトで数式として実行されないようテキストとして書く。
- **本アプリは投資助言ではない。** リバランス案は機械的な計算結果であり、売買は利用者が自分で行う。
- **過去のリポジトリ・旧アカウントとは無関係の新規実装。** 旧コードを持ち込まない。ポートフォリオの実データ（配分比率など）をリポジトリに含めない。

## Tauri Notes

- Rust 側のコマンドは `src-tauri/src/lib.rs` に集約し、フロントエンドからは `invoke` で呼ぶ。ポートフォリオは Rust が持ち、フロントエンドは `get_portfolio`（状態と不備の一覧）、`apply_portfolio_edit`（検証・保存は Rust）、`export_all`（全データの書き出し）、`get_overview`（カテゴリごとの評価額と比率。グラフ用の読み取り専用）、`get_account_balance`（口座内の商品バランス。読み取り専用）、`suggest_rebalance`（読み取り専用の提案）、`plan_import`（取り込みの対応づけの確認。読み取り専用）、表の取り込み（`load_table_file`、`load_table_text`、`plan_table_import`、`clear_table`）、保存先（`set_data_folder`、`icloud_data_folder`）、取得レシピ（`run_recipe`、`fetch_all_plan`、`apply_fetch_all`、`clear_fetch_runs`）、ブラウザで取得（`list_browsers`、`open_browser`、`browser_pages`、`fetch_browser_tables`、`use_fetched_table`、`close_browser`、`open_login_pages`（レシピが読むサイトのログインのページを開く。ログインは人が行う））と、bitFlyer のキー・残高のコマンドを使う。左パネル（`src/views/overview.ts`）、中央パネル（`holdings-table.ts`）、右パネル（`account-tabs.ts`）が表示する。
- 金額と比率は、フロントエンドでも十進数の文字列のまま扱い、数値型に変換しない。％と小数の変換は文字列の小数点の移動だけで行う（`src/decimal.ts`）。
- フロントエンドの単体テストは `src/*.test.ts`（`npm test`）。`@types/node` を入れないため、`tsc` の対象から外している。
- `src-tauri/tauri.conf.json` に CSP（本番 `csp` と開発用 `devCsp`）を設定してある。外部への接続や外部スクリプトが必要になったら、許可先を最小限で追加し、`docs/architecture.md` に理由を記録する。
- Cargo のルートは `Cargo.toml`（ワークスペース）。`Cargo.lock` はルートでコミットし、CI は `--locked` で実行する。
- `withGlobalTauri` は無効にしてある。フロントエンドは `@tauri-apps/api` を import して使い、`window.__TAURI__` に頼らない。

## CI / Local Development Commands

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

## Licensing

- コードは AGPL v3（SPDX: `AGPL-3.0-only`）。将来、商用ライセンスの提供と、自前サーバーアプリへの派生を想定している。
- 再ライセンスの余地を保つため、外部コントリビューションは `CONTRIBUTING.md` の Contribution Terms（準 CLA）に同意した PR のみ受け入れる。同意のチェックがない PR はマージしない。
- 追加する依存は AGPL v3 と両立するライセンスか確認する。商用ライセンスで配布できなくなるコピーレフト依存（GPL 系など）を入れない。
- 他者のコードを取り込む場合は、ライセンスと再ライセンス可否を確認してから入れる。

## Monetization

独自の課金システムは作らない。GitHub Sponsors と Buy Me a Coffee（`.github/FUNDING.yml`、README のバッジ）のみ。

## Tech Stack and Main Directories

**技術スタック:**

| 層 | 技術 |
|

## Tech Stack and Main Directories

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

**チーム規模:** 個人開発。アジャイルで迅速な意思決定を重視。
