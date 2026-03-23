#!/usr/bin/env nu
# Prints the current version from Cargo.toml.
# Usage: nu scripts/version.nu

open Cargo.toml --raw
| lines
| where { |l| $l =~ '^version\s*=' }
| first
| parse --regex 'version\s*=\s*"(?P<v>[^"]+)"'
| get v
| first
| print
