# Maintenance Prompts

ローカル LLM で `docs/` の更新案を作るためのプロンプト集。ローカル LLM は読み取り専用で使い、主担当 AI が根拠と実ファイルを照合してから保存する。

## Update architecture.md

~~~text
Tauri 2 アプリ（Rust + TypeScript）の docs/architecture.md の更新案を作成してください。

手順：
1. src/ と src-tauri/src/ のディレクトリ構造を確認する
2. src-tauri/Cargo.toml と package.json の依存を確認する
3. 既存の docs/architecture.md を読む
4. 次のフォーマットに沿って更新案を、根拠となるファイルパスとともに出力する
   - Overview（3行以内）
   - Entry Points（`パス` — 説明）
   - Directory Structure（ディレクトリと役割の表）
   - Layering Policy と Extraction Policy は変更せず保持する
   - Key Dependencies（主要な依存のみの表）

注意：ファイルレベルの詳細は書かない（file-map.md に委譲）。推測で書かず、確認できた内容だけを書く。
~~~

## Update file-map.md

~~~text
Tauri 2 アプリ（Rust + TypeScript）の docs/file-map.md の更新案を作成してください。

手順：
1. 追加・変更されたファイルを src/ と src-tauri/ から特定する
2. 各ファイルを読み、役割と主な依存先（import / use）を確認する
3. 既存の docs/file-map.md を読む
4. 次のフォーマットに沿って更新案を、根拠となるファイルパスとともに出力する
   - 先頭に「_最終更新: YYYY-MM-DD_」
   - 機能・モジュールごとの表（ファイル / 役割 / 主な依存先）

注意：未確認のファイルは記載しない。全ファイルを網羅する必要はない。
~~~
