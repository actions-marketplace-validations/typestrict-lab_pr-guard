Cart: replace addItem(item, quantity) with add(line)

`addItem` took the quantity as a separate positional argument, which made call sites easy to get
wrong. `add` now takes a full `LineItem` with its quantity. Bumped the major version since this
changes the public API; storefront and checkout need to update their calls.
