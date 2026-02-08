# AGENTS Instructions

This repository contains Rust code.
After making changes to the repository, run the following commands:

```
cargo check
cargo clippy -- -D warnings
cargo fmt
cargo test
```

## document

- docs/
  - DESIGN.md rm に関する実装状況や計画
  - DESIGN_CP.md cp に関する実装状況や計画

## コーディング規約

- コメントは日本語で記述すること
- ログやエラー文などは英語で記述すること
- 関数には責務や概要を記述しておくこと

## 自動テスト

- テストでは、関数名に加え、コメントでその意味を記述
- 外部依存はモックすること

## 手動テスト

- tmp/ ディレクトリ内で実ファイルやディレクトリの操作を行っても良い
