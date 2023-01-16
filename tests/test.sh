#!/usr/bin/env bash

set -e

TEMP_DIR="/tmp"
PROJECT_DIR=".."
TESTS_DIR="."
BIN="$PROJECT_DIR/target/debug/litua"

# TODO needs to work from project directory, make directories in output short

[ ! -f "$BIN" ] && echo "executable '$BIN' not found" && exit 1;

for EXAMPLE in enumeration literate-programming markup replacements ; do
  mkdir -p "$TEMP_DIR/$EXAMPLE"
  "$BIN" "$PROJECT_DIR/examples/$EXAMPLE/doc.txt" -o "$TEMP_DIR/$EXAMPLE.out.actual"
  "$BIN" "$PROJECT_DIR/examples/$EXAMPLE/doc.txt" --dump-hooks > "$TEMP_DIR/$EXAMPLE.hooks.actual"
  "$BIN" "$PROJECT_DIR/examples/$EXAMPLE/doc.txt" --dump-lexed > "$TEMP_DIR/$EXAMPLE.lexed.actual"
  "$BIN" "$PROJECT_DIR/examples/$EXAMPLE/doc.txt" --dump-parsed > "$TEMP_DIR/$EXAMPLE.parsed.actual"

  diff -u "$TESTS_DIR/$EXAMPLE.out.expected" "$TEMP_DIR/$EXAMPLE.out.actual"
  diff -u "$TESTS_DIR/$EXAMPLE.hooks.expected" "$TEMP_DIR/$EXAMPLE.hooks.actual"
  diff -u "$TESTS_DIR/$EXAMPLE.lexed.expected" "$TEMP_DIR/$EXAMPLE.lexed.actual"
  diff -u "$TESTS_DIR/$EXAMPLE.parsed.expected" "$TEMP_DIR/$EXAMPLE.parsed.actual"

  echo "Testcase '$EXAMPLE' passed"
done

rm -rf "$TEMP_DIR/*"
