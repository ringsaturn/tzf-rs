# Changelog

## [0.4.10] - 2025-02-01

* Bump the dependencies group across 1 directory with 6 updates by @dependabot in https://github.com/ringsaturn/tzf-rs/pull/159
* Bump the dependencies group across 1 directory with 5 updates by @dependabot in https://github.com/ringsaturn/tzf-rs/pull/161
* Bump the dependencies group with 4 updates by @dependabot in https://github.com/ringsaturn/tzf-rs/pull/162
* Bump tzf-rel-lite to 2025a

## [0.4.9] - 2024-09-10

* Bump the dependencies group with 3 updates by @dependabot in https://github.com/ringsaturn/tzf-rs/pull/153
* Bump the dependencies group with 3 updates by @dependabot in https://github.com/ringsaturn/tzf-rs/pull/154
* Bump the dependencies group across 1 directory with 6 updates by @dependabot in https://github.com/ringsaturn/tzf-rs/pull/157
* Bump tzf-rel data to [2024b](https://github.com/ringsaturn/tzf-rel/releases/tag/v0.0.2024-b), sync from https://github.com/evansiroky/timezone-boundary-builder/releases/tag/2024b

## [0.4.8] - 2024-07-31

- fix `DefaultFinder` auto nearby search
  - The simplified polygon data contains some empty areas where not covered by any timezone.It's not a bug but a limitation of the simplified algorithm.
  - To handle this, auto shift the point a little bit to find the nearest timezone.

## [0.4.7] - 2024-03-16

Update tzpb to 2024a

## [0.4.6] - 2024-03-13

* Bump runforesight/workflow-telemetry-action from 1 to 2 by @dependabot in https://github.com/ringsaturn/tzf-rs/pull/146
* Bump actions/upload-pages-artifact from 2 to 3 by @dependabot in https://github.com/ringsaturn/tzf-rs/pull/147
* Bump actions/deploy-pages from 3 to 4 by @dependabot in https://github.com/ringsaturn/tzf-rs/pull/145
* build: don't call cargo from build.rs by @grim7reaper in https://github.com/ringsaturn/tzf-rs/pull/150

## [0.4.5] - 2023-12-29

* feat(fuzzy): support get timezone names by @ringsaturn in https://github.com/ringsaturn/tzf-rs/pull/108
* Bump actions/checkout from 3 to 4 by @dependabot in https://github.com/ringsaturn/tzf-rs/pull/116
* Bump bytes from 1.4.0 to 1.5.0 by @dependabot in https://github.com/ringsaturn/tzf-rs/pull/117
* Bump prost-build from 0.11.9 to 0.12.0 by @dependabot in https://github.com/ringsaturn/tzf-rs/pull/115
* Bump cities-json from 0.4.0 to 0.5.0 by @dependabot in https://github.com/ringsaturn/tzf-rs/pull/124
* Bump cities-json from 0.5.0 to 0.5.1 by @dependabot in https://github.com/ringsaturn/tzf-rs/pull/126
* Update Rust toolchain channel to stable by @ringsaturn in https://github.com/ringsaturn/tzf-rs/pull/135
* Bump actions/upload-artifact from 3 to 4 by @dependabot in https://github.com/ringsaturn/tzf-rs/pull/136
* Bump tzf-rel to 2023d

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
