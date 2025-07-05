#!/bin/bash

# テスト用ディレクトリを作成
mkdir -p test_cp_demo/source_dir/subdir
echo "File 1 content" > test_cp_demo/source_dir/file1.txt
echo "File 2 content" > test_cp_demo/source_dir/file2.txt
echo "Subfile content" > test_cp_demo/source_dir/subdir/subfile.txt

echo "=== Test directory structure created ==="
find test_cp_demo -type f

echo -e "\n=== Testing cp without -R flag (should fail) ==="
cargo run --bin cp -- test_cp_demo/source_dir test_cp_demo/target_dir

echo -e "\n=== Testing cp with -r flag ==="
cargo run --bin cp -- -r test_cp_demo/source_dir test_cp_demo/target_dir

echo -e "\n=== Verifying copied structure ==="
find test_cp_demo/target_dir -type f

echo -e "\n=== Testing cp with -R flag (capital) ==="
rm -rf test_cp_demo/target_dir2
cargo run --bin cp -- -R test_cp_demo/source_dir test_cp_demo/target_dir2

echo -e "\n=== Verifying copied structure (capital R) ==="
find test_cp_demo/target_dir2 -type f

# クリーンアップ
rm -rf test_cp_demo