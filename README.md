# More Change Tokens Crate

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]

[crates-badge]: https://img.shields.io/crates/v/more-changetoken.svg
[crates-url]: https://crates.io/crates/more-changetoken
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/commonsensesoftware/more-rs-changetoken/blob/main/LICENSE

This library contains all of the fundamental abstractions for defining change tokens. Change tokens
are used to signal changes to consumers via a registered callback. This library also provides a
default change tokens:

- `NeverChangeToken` - will never change (e.g. _Null Object_ pattern)
- `SingleChangeToken` - will change at most once
- `SharedChangeToken` - shareable change token across multiple owners
- `AsyncSharedChangeToken` - thread-safe, shareable change token across multiple owners

## License

This project is licensed under the [MIT license].

[MIT license]: https://github.com/commonsensesoftware/more-rs-changetoken/blob/main/LICENSE