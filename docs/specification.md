# Specification

## Status

初期の雛形で、実装済みの機能はない。以下は目指す仕様であり、確定したものから追記する。
実装予定の作業は、このファイルではなく GitHub Issues で管理する。

## Scope

- 日本の証券口座（SBI 証券、楽天証券、マネックス証券、PayPay 証券、bitFlyer、Coincheck など）の資産を集約して表示する。
- 利用者が決めた目標ポートフォリオと比較し、リバランス案を計算して提示する。
- 売買は行わない。提示するだけで、実行は利用者が自分で行う。
- 資産の元データ（Excel など）を取り込まない。商品・口座・目標は、利用者がアプリ内で登録する。

## Data Flow

```
手入力 / 表形式テキストの取り込み / bitFlyer / Coincheck API → 保有（Rust） → リバランス提案（Rust） → UI
```

| 段階 | 内容 |
|---|---|
| 入力 | 手入力を基本とする。取り込みは CSV ファイルまたは貼り付けたテキスト、bitFlyer と Coincheck は REST API で、どれも保有額を上書きする |
| 正規化 | 取得元の名称を、利用者が登録した商品へ対応づけ（別名）、共通の保有モデルへ変換する |
| 計算 | 目標との差から提案を計算する。金額は decimal 型で扱う |
| 表示 | 全体のバランス（左の列）と、保有の一覧・全体の提案・口座ごとの分析と提案（副タブ）を、保有タブに出す |

## Constraints

- 証券会社へのログインはアプリ内で行わない。SBI 証券・マネックス証券はパスキーが必要で、Tauri の組み込み WebView では OS ごとに対応が不完全なため、ログインは利用者がシステムのブラウザで行う。
- ログイン情報（ID・パスワード・パスキー・2FA コード）は扱わず、保存しない。
- bitFlyer と Coincheck の API キー・シークレットは OS のキーチェーンに保存する。
- データはローカルのみに保存し、外部へ送信しない（保存先に OS が同期するフォルダを選んだ場合も、同期は OS が行い、アプリは何も送らない）。
- ロジックは Rust 側に置く（[architecture.md](architecture.md) の Layering Policy）。
- 証券会社が出力する形式は変わりうる。取り込みは列の対応づけを利用者が指定する汎用方式にし、形式が変わってもアプリの更新を要しないようにする。
- 個人利用に限る。各証券会社の利用規約を確認して使う。
- 投資助言ではない。


## Detail Files

仕様の詳細は、内容ごとに次のファイルに分けてある。

| ファイル | 内容 |
|---|---|
| [specification-import.md](specification-import.md) | 取得方法、表の取り込み（Table Import）、証券会社の出力（Brokerage Exports）、bitFlyer と Coincheck の残高 |
| [specification-browser.md](specification-browser.md) | ブラウザで取得（Browser Fetch） |
| [specification-recipes.md](specification-recipes.md) | 取得レシピ（Fetch Recipes） |
| [specification-presets.md](specification-presets.md) | 組み込みのレシピ（プリセット）、ログインのページ、結果の反映 |
| [specification-suggestions.md](specification-suggestions.md) | リバランスの計算（Rebalance Calculation）、提案モード（Suggestion Modes）、口座タブ |
| [specification-data.md](specification-data.md) | ポートフォリオのモデル、口座、保存（Storage）、エクスポート（Export） |
