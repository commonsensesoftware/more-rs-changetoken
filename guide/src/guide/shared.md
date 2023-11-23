{{#include links.md}}

# Shared Change Token

[`SharedChangeToken`] is one of, if not the most, commonly used change token. For all intents and purposes, [`SharedChangeToken`] functions the same as `Rc` or `Arc` without divulging that information. It also directly implements the [`ChangeToken`](default.md) trait, which means it can be used anywhere a [`ChangeToken`](default.md) is returned or accepted.

[`SharedChangeToken`] also defines `T: DefaultChangeToken`. This means `SharedChangeToken` is equivalent to `SharedChangeToken<DefaultChangeToken>` unless there is a more specific type on the left-hand side of an assignment. Although [`SingleChangeToken`](single.md) is more common in typical usage, if the default type `T` wasn't [`DefaultChangeToken`](default.md), it would be counter intuitive. [`SharedChangeToken`] can adapt over any other [`ChangeToken`] implementation.