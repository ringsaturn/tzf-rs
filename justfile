#!/usr/bin/env just --justfile
# ^ A shebang isn't required, but allows a justfile to be executed
#   like a script, with `./justfile test`, for example.

# https://github.com/casey/just

# VARIABLES
application := "fzf-tz"

# ALIASES
alias b := build
alias br := buildr
alias bra := buildra
alias fmt := format
alias r := release
alias update := upgrade
alias t := test
alias tp := testp

# SHORTCUTS AND COMMANDS

# Builds and documents the project - Default; runs if nothing else is specified
@default: check

# Check if it builds at all
@check: format
    cargo lcheck  --color 'always'

# Only compiles the project
@build: format changelog
   -git mit es
   cargo nextest run
   cargo lbuild --color 'always'

# Compile a release version of the project without moving the binaries
@buildr: format changelog
    cargo lbuild --release --color 'always'

# Compile a release version of the project for Apple ARM64 without moving the binaries
@buildra: format changelog
    cargo lbuild --release --color 'always' --target aarch64-apple-darwin
    cargo strip --target aarch64-apple-darwin

# Cleans and builds again
@rebuild: format changelog
    cargo clean
    cargo lbuild --color 'always'

# Updates the CHANGELOG.md file
@changelog:
   git-cliff --output {{invocation_directory()}}/CHANGELOG.md

# Cleans up the project directory
@clean:
    cargo clean
    -rm tree.txt > /dev/null 2>&1
    -rm graph.png > /dev/null 2>&1
    -rm debug.txt > /dev/null 2>&1
    -rm trace.txt > /dev/null 2>&1
    -rm bom.txt > /dev/null 2>&1
    -rm tests.txt > /dev/null 2>&1
    -rm tokei.txt > /dev/null 2>&1
    -rm {{application}}.log > /dev/null 2>&1

# Rebuilds the changelog
@cliff: changelog

# Documents the project, lints it, builds and installs the release version, and cleans up
@release: format changelog
    cargo lbuild --release  --color 'always'
    -cp {{invocation_directory()}}/target/release/{{application}} /usr/local/bin/
    cargo clean

# Documents the project, builds and installs the release version, and cleans up
@releasea: format changelog
    cargo lbuild --release  --color 'always' --target aarch64-apple-darwin
    cargo strip --target aarch64-apple-darwin
    cp {{invocation_directory()}}/target/aarch64-apple-darwin/release/{{application}} /usr/local/bin/
    cargo clean

# Build the documentation
@doc:
    cargo doc --no-deps

# Documents the project
@docs: format
    cargo doc --no-deps
    cargo depgraph | dot -Tpng > graph.png
    cargo tree > tree.txt
    cargo bom > bom.txt
    cargo nextest list | tee tests.txt
    tokei | tee tokei.txt
    cargo outdated

# Documents the project and all dependencies
@doc-all: format
    cargo doc
    cargo depgraph | dot -Tpng > graph.png
    cargo tree > tree.txt
    cargo bom > bom.txt
    cargo nextest list | tee tests.txt
    tokei | tee tokei.txt
    cargo outdated

# Formats the project source files
@format:
    cargo fmt -- --emit=files

# Tests the project
@test:
    cargo nextest run

# Tests the project with output
@testp:
    cargo nextest run --no-capture

# Checks the project for inefficiencies and bloat
@inspect: format doc lint spell
    cargo deny check
    cargo geiger
    cargo bloat
    cargo pants

# Checks for potential code improvements
@lint:
    cargo lclippy -- -W clippy::pedantic -W clippy::nursery -W clippy::unwrap_used

# Checks for potential code improvements and fixes what it can
@lintfix:
    cargo lclippy --fix -- -W clippy::pedantic -W clippy::nursery -W clippy::unwrap_used


# Initialize directory for various services such as cargo deny
@init:
    -cp ~/CloudStation/Source/_Templates/deny.toml {{invocation_directory()}}/deny.toml
    -cp ~/CloudStation/Source/_Templates/main_template.rs {{invocation_directory()}}/src/main.rs
    -cp ~/CloudStation/Source/_Templates/cliff.toml {{invocation_directory()}}/cliff.toml
    -cargo add clap --features cargo color
    -cargo add log
    -cargo add env_logger
    -echo "# {{application}}\n\n" > README.md
    -git mit-install
    -git mit-config lint enable subject-line-not-capitalized
    -git mit-config lint enable subject-line-ends-with-period
    -git mit-config lint enable not-conventional-commit
    -git mit-config lint disable not-emoji-log
    -git mit-config mit set es "Even Solberg" even.solberg@gmail.com
    -git mit es
    -git remote add {{application}} https://github.com/evensolberg/{{application}}
    -git commit -m doc:Initial
    -git tag Initial
    -git cliff --init
    -cp ~/CloudStation/Source/_Templates/cliff.toml {{invocation_directory()}}/
    -scaffold add cli

# Re-initialize the directory for various services -- stripped down version of init

@reinit:
    git mit-install
    git mit-config lint enable subject-line-not-capitalized
    git mit-config lint enable subject-line-ends-with-period
    git mit-config mit set es "Even Solberg" even.solberg@gmail.com
    git mit es
    git cliff --init
    cp ~/CloudStation/Source/_Templates/cliff.toml {{invocation_directory()}}/

# Read the documentation
@read:
    open file://{{invocation_directory()}}/target/doc/{{application}}/index.html

# Builds (if necessary) and runs the project
@run:
    cargo lrun  --color 'always'

# Build and run with a --help parameter
@runh:
    cargo lrun  --color 'always' -- --help

# Build and run with a --debug parameter
@rund:
    cargo lrun  --color 'always' -- --debug

# Build and run with a --debug parameter, tee to debug.txt
@rundt:
    cargo lrun  --color 'always' -- --debug | tee debug.txt

# Build and run with double --debug parameters
@rundd:
    cargo lrun  --color 'always' -- --debug --debug

# Build and run with double --debug parameters, tee to trace.txt
@runddt:
    cargo lrun  --color 'always' -- --debug --debug | tee trace.txt

# Spellcheck the documents except CHANGELOG
@spell:
    typos --exclude CHANGELOG.md -c ~/CloudStation/Automation/_typos.toml

# Check for new versions of crates and upgrade accordingly
@upgrade:
    cargo update
    cargo upgrade --workspace

# Copy this settings files to the templates directory
@just:
    -sed "s#{{application}}#my_application#" justfile > ~/CloudStation/Source/_Templates/justfile.template
    -cp {{invocation_directory()}}/deny.toml ~/CloudStation/Source/_Templates/deny.toml
    -cp {{invocation_directory()}}/cliff.toml ~/CloudStation/Source/_Templates/cliff.toml

# Check, but verbose
@checkv:
    cargo lcheck --color 'always' --verbose

# Install the relevant cargo add-ons used in this file
@install:
    -cargo install cargo-limit
    -cargo install cargo-geiger
    -cargo install cargo-depgraph
    -cargo install cargo-audit
    -cargo install cargo-bloat
    -cargo install cargo-edit
    -cargo install cargo-strip
    -cargo install --locked cargo-outdated
    -cargo install tokei
    -cargo install cargo-semver --vers 1.0.0-alpha.3
    -cargo install cargo-deny
    -cargo install git-cliff
    -cargo install cargo-nextest
    -cargo install cargo-pants
    -brew install PurpleBooth/repo/git-mit
    -brew install graphviz
    -cp ~/CloudStation/Source/_Templates/deny.toml {{invocation_directory()}}/deny.toml

# Testing actions

# Run the program with a bunch of parameters to test things
@runit:
    -rm my_application.log
    target/debug/my_application \
        --pfc folder.jpg --pfc Front.jpg \
        --pbc Back.jpg --pbc Back-Cover.jpg \
        --psf Artwork --psf "." --psf ".." \
        --pms 300 \
        --pf cover-small.jpg --pb back-small.jpg  \
        -l my_application/debug.yaml \
        music/01-13\ Surf\'s\ Up.flac \
        -r
