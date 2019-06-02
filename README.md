# json-query

[![crates.io](https://img.shields.io/crates/v/json-query.svg)](https://crates.io/crates/json-query)
[![crates.io](https://img.shields.io/crates/d/json-query.svg)](https://crates.io/crates/json-query)
[![docs.rs](https://docs.rs/json-query/badge.svg)](https://docs.rs/json-query)

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
[jq-sys]: https://github.com/onelson/jq-sys
[jq-sys-building]: https://github.com/onelson/jq-sys#building
[jq-src]: https://github.com/onelson/jq-src

# Changelog

## 0.3.0 (2019-06-01)

- Added `json_query::compile()`. Compile a jq program, then reuse it, running
  it against several inputs.

## 0.2.1 (2019-06-01)

- [#1] Enabled `bundled` feature when building on docs.rs.

## 0.2.0 (2019-02-18)

- Updates [jq-sys] dep to v0.2.0 for better build controls.
- Settles on 2015 edition style imports (for now).

Breaking Changes:

- `bundled` feature is no longer enabled by default.


## 0.1.1 (2019-01-14)

* Added extra links to cargo manifest.
* Added some basic docs.
* Added a `bundled` feature to opt in or out of using the bundled source.

## 0.1.0 (2019-01-13)

Initial release.

[#1]: https://github.com/onelson/json-query/issues/1
