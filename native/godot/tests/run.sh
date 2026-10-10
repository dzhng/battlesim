#!/bin/sh
# Runs every headless test (or those named) with a clean environment. A test
# passes only when Godot exits 0 and prints its result line with no failures:
# a parse error also exits 0, but prints no result line.
set -u
cd "$(dirname "$0")/.."
status=0
for test in ${@:-tests/*_test.gd}; do
	name=$(basename "$test")
	output=$(env -i HOME="$HOME" PATH="$PATH" godot --headless --path . --script "res://tests/$name" 2>&1)
	code=$?
	if [ $code -eq 0 ] && printf '%s\n' "$output" | grep -q "^{\"failures\":\[\],\"test\":\"$name\"}$"; then
		echo "ok   $name"
	else
		echo "FAIL $name (exit $code)"
		printf '%s\n' "$output" | grep -E 'ERROR|failures' | grep -v -E 'leaked|Leaked|still in use|PagedAllocator' | head -20
		status=1
	fi
done
exit $status
