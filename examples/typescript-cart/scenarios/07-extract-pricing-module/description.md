Move totals computation into src/pricing.ts

The checkout service wants to price a list of lines without building a Cart (for reorders), so the
totals math moves to `computeTotals` in a new pricing module and `Cart.totals` delegates to it.
`Totals` is re-exported from cart.ts so existing imports keep working. No behavior change; added
unit tests for the new module.
