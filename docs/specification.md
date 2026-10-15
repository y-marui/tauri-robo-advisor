# Specification

## Status

初期の雛形で、実装済みの機能はない。以下は目指す仕様であり、確定したものから追記する。
実装予定の作業は、このファイルではなく GitHub Issues で管理する。

## Scope

- 日本の証券口座（SBI 証券、マネックス証券、bitFlyer）の資産を集約して表示する。
- 利用者が決めた目標ポートフォリオと比較し、リバランス案を計算して提示する。
- 売買は行わない。提示するだけで、実行は利用者が自分で行う。

## Data Flow

```
Broker site / API → Acquisition (Rust) → Normalized asset model → Rebalance logic (Rust) → UI
```

| 段階 | 内容 |
|---|---|
| 取得 | SBI 証券・マネックス証券は、Tauri の WebView でユーザー本人がログインした後のページから資産表を読み取る。bitFlyer は REST API を使う |
| 正規化 | 各社の取得結果を共通の資産モデルへ変換する |
| 計算 | 目標配分との差から売買案を計算する。金額は decimal 型で扱う |
| 表示 | 結果を一覧で表示する |

## Constraints

- ログイン情報（ID・パスワード・2FA コード）は保存しない。
- bitFlyer の API キー・シークレットは OS のキーチェーンに保存する。
- データはローカルのみに保存し、外部へ送信しない。
- ロジックは Rust 側に置く（[architecture.md](architecture.md) の Layering Policy）。
- 証券会社の画面構造が変わると取得が壊れる。取得部分は差し替え可能な単位に閉じ込める。
- 個人利用に限る。各証券会社の利用規約を確認して使う。
- 投資助言ではない。
