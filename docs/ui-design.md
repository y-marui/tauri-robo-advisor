# UI Design

## Status

雛形の画面（Tauri のデモ）のみ。機能ごとの画面は未設計。

## Principles

[UI_GUIDELINES.md](dev-charter/UI_GUIDELINES.md) に従う。

- ライト・ダーク・システムの各モードに対応し、ユーザーが設定から選べるようにする。
- 配色: ライトは背景 `#FFFFFF`・文字 `#4e454a`・強調 `#000000`。ダークは背景 `#000000`・文字 `#bab1b6`・強調 `#FFFFFF`。
- システムカラーとネイティブ UI コンポーネントを優先し、テーマに合わせて CSS の `color-scheme` を設定する。
- Unicode の絵文字を UI パーツに使わない。アイコンは Google Material Symbols を第一選択にする。
- UI の文言は英語を開発言語とし、他言語は [LOCALIZATION_POLICY.md](dev-charter/LOCALIZATION_POLICY.md) に従って追加する。
- 表示と入力のみを扱い、計算や検証は Rust 側に置く。
