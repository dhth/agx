set dotenv-load := true

alias a := all
alias b := build
alias c := check
alias d := deny
alias f := fmt
alias fc := fmt-check
alias i := install
alias l := lint
alias lf := lint-fix
alias r := run
alias re := review
alias t := test

default:
    just --choose

aud:
    cargo audit --all-targets

build:
    cargo build

check:
    cargo check --all-targets

deny:
    cargo deny check --hide-inclusion-graph

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

install:
    cargo install --path .

lint:
    cargo clippy --all-targets

lint-fix:
    cargo clippy --fix  --allow-dirty --allow-staged

run *FLAGS:
    cargo run -- {{FLAGS}}

review *FLAGS:
    cargo insta test --review {{FLAGS}}

test:
    cargo test

tail-logs:
    tail -n 100 ~/.local/state/agx/agx.log

all:
    cargo check --all-targets
    cargo fmt --all
    cargo clippy --all-targets
    cargo test
