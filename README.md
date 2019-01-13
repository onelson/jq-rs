# json-query

This rust crate provides programmatic access to [jq] 1.6 via its C api.

> Note that libjq is provided and linked statically by [jq-sys] and [jq-src]
> which require having autotools and gcc in `PATH` to build.

By leveraging [jq] we can extract and transform data from a json string
using jq's filtering dsl.

```rust
use json_query;
// ...

let res = json_query::run(".name", r#"{"name": "test"}"#);
assert_eq!(res, Ok("\"test\"".to_string()));
```

The return values from the run method are json strings, and as such will need
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

[jq]: https://github.com/stedolan/jq
[serde_json]: https://github.com/serde-rs/json
[jq-sys]: https://github.com/onelson/jq-sys
[jq-src]: https://github.com/onelson/jq-src
