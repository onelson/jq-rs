# jq-rs

[![crates.io](https://img.shields.io/crates/v/jq-rs.svg)](https://crates.io/crates/jq-rs)
[![crates.io](https://img.shields.io/crates/d/jq-rs.svg)](https://crates.io/crates/jq-rs)
[![docs.rs](https://docs.rs/jq-rs/badge.svg)](https://docs.rs/jq-rs)

> Prior to v0.4.0 this crate was named [json-query].

This rust crate provides access to [jq] 1.6 via the `libjq` C API (rather than
"shelling out").

By leveraging [jq] we can extract data from json strings using `jq`'s dsl.

This crate requires Rust **1.32** or above.

## Usage

The interface provided by this crate is very basic. You supply a jq program
string and a string to run the program over.

```rust
use jq_rs;
// ...

let res = jq_rs::run(".name", r#"{"name": "test"}"#);
assert_eq!(res, Ok("\"test\"\n".to_string()));
```

In addition to running one-off programs with `jq_rs::run()`, you can also
use `jq_rs::compile()` to compile a jq program and reuse it with
different inputs.

```rust
use jq_rs;

let tv_shows = r#"[
    {"title": "Twilight Zone"},
    {"title": "X-Files"},
    {"title": "The Outer Limits"}
]"#;

let movies = r#"[
    {"title": "The Omen"},
    {"title": "Amityville Horror"},
    {"title": "The Thing"}
]"#;

let mut program = jq_rs::compile("[.[].title] | sort").unwrap();

assert_eq!(
    &program.run(tv_shows).unwrap(),
    "[\"The Outer Limits\",\"Twilight Zone\",\"X-Files\"]\n"
);

assert_eq!(
    &program.run(movies).unwrap(),
    "[\"Amityville Horror\",\"The Omen\",\"The Thing\"]\n",
);
```

### A note on perf

While the benchmarks are far from exhaustive, they indicate that much of the
runtime of a simple jq program goes to the compilation. In fact, the compilation
is _quite expensive_.

```
run one off             time:   [48.594 ms 48.689 ms 48.800 ms]
Found 6 outliers among 100 measurements (6.00%)
  3 (3.00%) high mild
  3 (3.00%) high severe

run pre-compiled        time:   [4.0351 us 4.0708 us 4.1223 us]
Found 15 outliers among 100 measurements (15.00%)
  6 (6.00%) high mild
  9 (9.00%) high severe
```

If you have a need to run the same jq program multiple times it is
_highly recommended_ to retain a pre-compiled `JqProgram` and reuse it.

### Handling Output

The return values from jq are _strings_ since there is no certainty that the
output will be valid json. As such the output will need to be parsed if you want
to work with the actual data types being represented.

In such cases you may want to pair this crate with [serde_json] or similar.

For example, here we want to extract the numbers from a set of objects:

```rust
use jq_rs;
use serde_json::{self, json};

// ...

let data = json!({
    "movies": [
        { "title": "Coraline", "year": 2009 },
        { "title": "ParaNorman", "year": 2012 },
        { "title": "Boxtrolls", "year": 2014 },
        { "title": "Kubo and the Two Strings", "year": 2016 },
        { "title": "Missing Link", "year": 2019 }
    ]
});

let query = "[.movies[].year]";
// program output as a json string...
let output = jq_rs::run(query, &data.to_string()).unwrap();
// ... parse via serde
let parsed: Vec<i64> = serde_json::from_str(&output).unwrap();

assert_eq!(vec![2009, 2012, 2014, 2016, 2019], parsed);
```

Barely any of the options or flags available from the [jq] cli are exposed
currently.
Literally all that is provided is the ability to execute a _jq program_ on a blob
of json.
Please pardon my dust as I sort out the details.

## Linking to libjq

This crate requires access to `libjq` at build and/or runtime depending on the
your choice.

When the `bundled` feature is enabled (**off by default**) `libjq` is provided
and linked statically to your crate by [jq-sys] and [jq-src]. Using this feature
requires having autotools and gcc in `PATH` in order for the to build to work.

Without the `bundled` feature, _you_ will need to ensure your crate
can link to `libjq` in order for the bindings to work.

You can choose to compile `libjq` yourself, or perhaps install it via your
system's package manager.
See the [jq-sys building docs][jq-sys-building] for details on how to share
hints with the [jq-sys] crate on how to link.

[jq]: https://github.com/stedolan/jq
[serde_json]: https://github.com/serde-rs/json
[jq-rs]: https://crates.io/crates/jq-rs
[json-query]: https://crates.io/crates/json-query
[jq-sys]: https://github.com/onelson/jq-sys
[jq-sys-building]: https://github.com/onelson/jq-sys#building
[jq-src]: https://github.com/onelson/jq-src

# Changelog

## v0.4.0 (Unreleased)

Breaking Changes

- Renamed crate from `json-query` to `jq-rs` ([#12]).
- Adopted 2018 edition. The minimum supported rust version is now **1.32** ([#14]).
- Output from jq programs now includes a trailing newline, just like the output
  from the `jq` binary ([#6]).

## v0.3.1 ([2019-06-04](https://github.com/onelson/json-query/compare/v0.3.0..v0.3.1 "diff"))

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

## 0.1.0 (2019-01-13)

Initial release.

[#1]: https://github.com/onelson/json-query/issues/1
[#3]: https://github.com/onelson/json-query/issues/3
[#4]: https://github.com/onelson/json-query/issues/4
[#6]: https://github.com/onelson/jq-rs/pull/6
[#10]: https://github.com/onelson/json-query/issues/10
[#12]: https://github.com/onelson/jq-rs/issues/12
[#14]: https://github.com/onelson/jq-rs/issues/14
