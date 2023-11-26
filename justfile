build:
  cargo build

build-refactor:
  # requires cargo-limit to be installed
  reset
  (cargo lbuild --color=always 2>&1) | less -R

watchexec target:
  watchexec \
    -c \
    -e toml,rs,proto \
    -w justfile \
    -w Cargo.toml \
    -w crates/router/src \
    -w crates/router/Cargo.toml \
    -w crates/router/proto \
    -w crates/router/build.rs \
    -w crates/cmdr/src \
    -w crates/cmdr/Cargo.toml \
    --restart \
    just {{target}}

we-build-refactor:
  just watchexec build-refactor

we-build:
  just watchexec build
