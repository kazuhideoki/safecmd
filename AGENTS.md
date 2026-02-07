# AGENTS Instructions

This repository contains Rust code.
After making changes to the repository, run the following commands:

```
cargo check
cargo clippy -- -D warnings
cargo fmt
cargo test
```

## コーディング規約

- コメントは日本語で記述すること
- ログやエラー文などは英語で記述すること
- 関数には責務や概要を記述しておくこと

## テスト

- テストでは、関数名に加え、コメントでその意味を記述
- 外部依存はモックすること
