# Changelog

## [0.4.4] - 2023-08-19

- Various fixes by @evensolberg
  [#102](https://github.com/ringsaturn/tzf-rs/pull/102)
  - Cleaned up the code a little by running Clippy in pedantic mode
    - Made the code a little more Rust-like by implmenting the `Default` trait
      for some structs.
  - Added `--lon` as an alias for `--lng`
  - Allow negative numbers for latitude and longitude, meaning we can go to the
    southern and western hemispheres.

## [0.4.3] - 2023-07-31

Update docs

## [0.4.2] - 2023-07-31

- Update Cargo lock

## [0.4.1] - 2023-07-13

- Make clap an optional dependency by @wildwestrom in
  [#69](https://github.com/ringsaturn/tzf-rs/pull/69)
- Bump deps

## [0.4.0] - 2023-05-27

- Bump tzf-rel to v0.0.2023-b and add data_version method
  [#57](https://github.com/ringsaturn/tzf-rs/pull/57)

## [0.3.1] - 2023-05-21

Bump deps

## [0.3.0] - 2023-01-29

- Fix [preindex bug](https://github.com/ringsaturn/tzf/issues/76) by bump
  tzf-rel version
- Bump other deps

## [0.2.1] - 2023-01-25

### Bug Fixes

- Pprof can not build on windows (#36)

## [0.2.0] - 2023-01-19

- support get all matched timezone names by @ringsaturn in
  [#33](https://github.com/ringsaturn/tzf-rs/pull/33)
