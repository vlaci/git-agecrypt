coverage := "0"

export RUSTC_BOOTSTRAP := if coverage =~ "(1|true)" { "1" } else { "" }
export RUSTFLAGS := if coverage =~ "(1|true)" { "-Zinstrument-coverage" } else { "" }

test:
    cargo ltest

dev:
    # For coverage
    rustup component add llvm-tools-preview

covreport:
    grcov . -s . --binary-path ./target/debug -t html --branch --ignore-not-existing -o ./target/debug/coverage

build:
    cargo lbuild

clippy:
    cargo lclippy --all

fmt:
    cargo fmt --all

check: fmt clippy test

run:
    cargo run

watch +COMMAND='ltest':
    cargo watch --ignore "*.profraw" --clear --exec "{{COMMAND}}"
