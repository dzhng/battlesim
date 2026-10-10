#!/bin/sh
# Runs every headless test (or those named) with a clean environment. A test
# passes only when Godot exits 0, prints its result line with no failures and
# logs no script error: a parse error also exits 0 without a result line, and a
# runtime error inside a called function returns null and lets the caller go on.
set -u
cd "$(dirname "$0")/.."
status=0
for test in ${@:-tests/*_test.gd}; do
	name=$(basename "$test")
	output=$(env -i HOME="$HOME" PATH="$PATH" godot --headless --path . --script "res://tests/$name" 2>&1)
	code=$?
	if [ $code -eq 0 ] && printf '%s\n' "$output" | grep -q "^{\"failures\":\[\],\"test\":\"$name\"}$" && ! printf '%s\n' "$output" | grep -q "SCRIPT ERROR"; then
		echo "ok   $name"
	else
		echo "FAIL $name (exit $code)"
		printf '%s\n' "$output" | grep -E 'ERROR|failures' | grep -v -E 'leaked|Leaked|still in use|PagedAllocator' | head -20
		status=1
	fi
done
exit $status
