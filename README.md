# json-query

[![crates.io](https://img.shields.io/crates/v/json-query.svg)](https://crates.io/crates/json-query)
[![crates.io](https://img.shields.io/crates/d/json-query.svg)](https://crates.io/crates/json-query)
[![docs.rs](https://docs.rs/json-query/badge.svg)](https://docs.rs/json-query)

> **Notice**
>
> **Release 0.3.1 marks the end of the line for `json-query`.**
> **Future releases will be published under the new name of [jq-rs].**

This rust crate provides programmatic access to [jq] 1.6 via its C api.

By leveraging [jq] we can extract and transform data from a json string
using jq's filtering dsl.

```rust
use json_query;
// ...

let res = json_query::run(".name", r#"{"name": "test"}"#);
assert_eq!(res, Ok("\"test\"".to_string()));
```

In addition to running one-off programs with `json_query::run()`, you can also
use `json_query::compile()` to compile a jq program and reuse it with
different inputs.

```rust
use json_query;

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

let mut program = json_query::compile("[.[].title] | sort").unwrap();

assert_eq!(
    r#"["The Outer Limits","Twilight Zone","X-Files"]"#,
    &program.run(tv_shows).unwrap()
);

assert_eq!(
    r#"["Amityville Horror","The Omen","The Thing"]"#,
    &program.run(movies).unwrap()
);
```

The return values from the run methods are json strings, and as such will need
to be parsed if you want to work with the actual data types being represented.
As such, you may want to pair this crate with [serde_json] or similar.

For example, here we want to extract the numbers from a set of objects:

```rust
use json_query;
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
let output = json_query::run(query, &data.to_string()).unwrap();
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

When the `bundled` feature is enabled (**off by default**) `libjq` is provided and
linked statically by [jq-sys] and [jq-src]
which require having autotools and gcc in `PATH` to build.

If you disable the `bundled` feature, you will need to ensure your crate
links to `libjq` in order for the bindings to work.

See the [jq-sys building docs][jq-sys-building] for details on how to share
hints with the [jq-sys] crate on how to link.

> Note that it may be required to `cargo clean` when switching between building with
> `bundled` enabled or disabled.
>
> I can't explain it, but sometimes the `bundled` build will break if you don't give the
> out dir a good scrubbing.

[jq]: https://github.com/stedolan/jq
[serde_json]: https://github.com/serde-rs/json
[jq-rs]: https://crates.io/crates/jq-rs
[jq-sys]: https://github.com/onelson/jq-sys
[jq-sys-building]: https://github.com/onelson/jq-sys#building
[jq-src]: https://github.com/onelson/jq-src

# Changelog

## v0.3.1 ([2019-06-04](https://github.com/onelson/json-query/compare/v0.3.0..v0.3.1 "diff"))

**Note: This is final release with the name `json-query`.
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
[#10]: https://github.com/onelson/json-query/issues/10
