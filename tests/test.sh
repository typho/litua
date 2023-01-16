#!/usr/bin/env bash

set -e

TEMP_DIR="/tmp"
PROJECT_DIR="."
TESTS_DIR="tests"
BIN="$PROJECT_DIR/target/debug/litua"

[ ! -f "$BIN" ] && echo "executable '$BIN' not found" && exit 1;

for EXAMPLE in enumeration literate-programming markup replacements ; do
  mkdir -p "$TEMP_DIR/$EXAMPLE"
  "$BIN" "$PROJECT_DIR/examples/$EXAMPLE/doc.lit" -o "$TEMP_DIR/$EXAMPLE.out.actual"
  "$BIN" "$PROJECT_DIR/examples/$EXAMPLE/doc.lit" --dump-hooks > "$TEMP_DIR/$EXAMPLE.hooks.actual"
  "$BIN" "$PROJECT_DIR/examples/$EXAMPLE/doc.lit" --dump-lexed > "$TEMP_DIR/$EXAMPLE.lexed.actual"
  "$BIN" "$PROJECT_DIR/examples/$EXAMPLE/doc.lit" --dump-parsed > "$TEMP_DIR/$EXAMPLE.parsed.actual"

  diff -u "$TESTS_DIR/$EXAMPLE.out.expected" "$TEMP_DIR/$EXAMPLE.out.actual"
  diff -u "$TESTS_DIR/$EXAMPLE.hooks.expected" "$TEMP_DIR/$EXAMPLE.hooks.actual"
  diff -u "$TESTS_DIR/$EXAMPLE.lexed.expected" "$TEMP_DIR/$EXAMPLE.lexed.actual"
  diff -u "$TESTS_DIR/$EXAMPLE.parsed.expected" "$TEMP_DIR/$EXAMPLE.parsed.actual"

  echo "Testcase '$EXAMPLE' passed"
done

rm -rf "$TEMP_DIR/*"
