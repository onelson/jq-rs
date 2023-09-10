# Changelog

## v0.4.1 ([2019-08-17](https://github.com/onelson/jq-rs/compare/v0.4.0..v0.4.1 "diff"))

Additions

- Implements `std::error::Error + Send + 'static` for `jq_rs::Error` to better
  integrate with popular error handling crate [error-chain] and others ([#22]).

## v0.4.0 ([2019-07-06](https://github.com/onelson/jq-rs/compare/v0.3.1..v0.4.0 "diff"))

Breaking Changes

- Renamed crate from `json-query` to `jq-rs` ([#12]).
- Adopted 2018 edition. The minimum supported rust version is now **1.32** ([#14]).
- Output from jq programs now includes a trailing newline, just like the output
  from the `jq` binary ([#6]).
- Added custom `Error` and `Result` types, returned from fallible
  functions/methods in this crate ([#8]).

## v0.3.1 ([2019-07-04](https://github.com/onelson/json-query/compare/v0.3.0..v0.3.1 "diff"))

**Note: This is final release with the name [json-query].
Future releases will be published as [jq-rs].**

Bugfixes

- Fixed issue where newlines in output were not being preserved correctly ([#3]).
- Resolved a memory error which could cause a crash when running a jq program
  which could attempt to access missing fields on an object ([#4]).
- Fixed some memory leaks which could occur during processing ([#10]).

## v0.3.0 ([2019-06-01](https://github.com/onelson/json-query/compare/v0.2.1..v0.3.0 "diff"))

- Added `json_query::compile()`. Compile a jq program, then reuse it, running
  it against several inputs.

## v0.2.1 ([2019-06-01](https://github.com/onelson/json-query/compare/v0.2.0..v0.2.1 "diff"))

- [#1] Enabled `bundled` feature when building on docs.rs.

## v0.2.0 ([2019-02-18](https://github.com/onelson/json-query/compare/v0.1.1..v0.2.0 "diff"))

- Updates [jq-sys] dep to v0.2.0 for better build controls.
- Settles on 2015 edition style imports (for now).

Breaking Changes:

- `bundled` feature is no longer enabled by default.


## v0.1.1 ([2019-01-14](https://github.com/onelson/json-query/compare/v0.1.0..v0.1.1 "diff"))

- Added extra links to cargo manifest.
- Added some basic docs.
- Added a `bundled` feature to opt in or out of using the bundled source.

## v0.1.0 (2019-01-13)

Initial release.

[jq]: https://github.com/stedolan/jq
[serde_json]: https://github.com/serde-rs/json
[jq-rs]: https://crates.io/crates/jq-rs
[json-query]: https://crates.io/crates/json-query
[jq-sys]: https://github.com/onelson/jq-sys
[jq-sys-building]: https://github.com/onelson/jq-sys#building
[jq-src]: https://github.com/onelson/jq-src
[error-chain]: https://crates.io/crates/error-chain

[#1]: https://github.com/onelson/json-query/issues/1
[#3]: https://github.com/onelson/json-query/issues/3
[#4]: https://github.com/onelson/json-query/issues/4
[#6]: https://github.com/onelson/jq-rs/pull/6
[#8]: https://github.com/onelson/jq-rs/pull/8
[#10]: https://github.com/onelson/json-query/issues/10
[#12]: https://github.com/onelson/jq-rs/issues/12
[#14]: https://github.com/onelson/jq-rs/issues/14
[#22]: https://github.com/onelson/jq-rs/pull/22
