build:
  cargo build

build-refactor:
  # requires cargo-limit to be installed
  reset
  (cargo lbuild --color=always 2>&1) | less -R

watchexec target:
  watchexec \
    -c \
    -e toml,rs \
    -w justfile \
    -w Cargo.toml \
    -w crates/twiymcbof-router/src \
    -w crates/twiymcbof-router/Cargo.toml \
    -w crates/twiymcbof-cmdr/src \
    -w crates/twiymcbof-cmdr/Cargo.toml \
    --restart \
    just {{target}}

we-build-refactor:
  just watchexec build-refactor

we-build:
  just watchexec build
