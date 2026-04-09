#!/usr/bin/env bash

set -euo pipefail

: "${VERSION:?VERSION must be set}"
: "${TODAY:?TODAY must be set}"

if [[ "$VERSION" != *-* ]]; then
  perl -0pi -e '
    s{<a href="https://github\.com/Carabryx/DotenvPP/releases(?:/latest)?"><img src="https://img\.shields\.io/[^"]+" alt="[^"]*" /></a>}{<a href="https://github.com/Carabryx/DotenvPP/releases/latest"><img src="https://img.shields.io/github/v/release/Carabryx/DotenvPP?label=release&color=171717" alt="Latest release" /></a>}
  ' README.md
fi

perl -0pi -e '
  my $version = $ENV{VERSION};
  my $today = $ENV{TODAY};
  my $marker = "## [Unreleased]\n";
  my $section = "## [$version] - $today";
  my $version_link = "[$version]: https://github.com/Carabryx/DotenvPP/releases/tag/v$version";
  my $unreleased_link = "[Unreleased]: https://github.com/Carabryx/DotenvPP/compare/v$version...HEAD";

  die "CHANGELOG.md is missing ## [Unreleased]\n" unless index($_, $marker) >= 0;

  if (index($_, $section) < 0) {
    s/\Q$marker\E/${marker}\n${section}\n\n/ or die "Failed to insert release section\n";
  }

  if (index($_, $unreleased_link) < 0) {
    s/^\[Unreleased\]: .*$/$unreleased_link/m or die "Missing expected changelog pattern\n";
  }

  if (index($_, $version_link) < 0) {
    s/^\[Unreleased\]: .*$/$&\n$version_link/m or die "Missing expected changelog pattern\n";
  }
' CHANGELOG.md
