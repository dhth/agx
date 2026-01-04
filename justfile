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

@default:
    just --choose

aud:
    cargo audit --all-targets

build:
    cargo build

build-linux:
    @if ! command -v cargo-zigbuild &> /dev/null; then \
        echo "Error: cargo-zigbuild not found. Install it with: cargo install cargo-zigbuild" >&2; \
        exit 1; \
    fi
    cargo zigbuild --target x86_64-unknown-linux-musl

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
    cargo run -- {{ FLAGS }}

review *FLAGS:
    cargo insta test --review {{ FLAGS }}

test:
    cargo test

tail-logs:
    tail -n 100 ~/.local/state/agx/agx.log

docker-up:
    @if [ ! -f "target/x86_64-unknown-linux-musl/debug/agx" ]; then \
        echo "Error: linux binary not found. Run 'just build-linux' first" >&2; \
        exit 1; \
    fi
    cd local && docker compose up -d

[working-directory('local')]
docker-shell:
    docker compose exec dev /usr/bin/env bash

[working-directory('local')]
docker-down:
    docker compose down

all:
    just check
    just fmt
    just lint
    just test

curl-events:
    curl -Ns http://127.0.0.1:4880/api/debug/events | tee ~/.local/state/agx/events.json

rm-events:
    rm ~/.local/state/agx/events.json

[working-directory: 'src/debug/client']
debug-check:
    gleam check

[working-directory: 'src/debug/client']
debug-build:
    gleam run -m lustre/dev build agx_debug

[working-directory: 'src/debug/client']
debug-run:
    gleam run -m lustre/dev start

[working-directory: 'src/debug/client']
debug-fmt:
    gleam format src

debug-all:
    just debug-check
    just debug-fmt
    just debug-build

# for AI agents
tail-events:
    @if [ ! -f "$HOME/.local/state/agx/events.json" ]; then \
        echo 'Error: events.json is not created; ask the user to run "just curl-events" first' >&2; \
        exit 1; \
    fi

    tail -n 10 ~/.local/state/agx/events.json
