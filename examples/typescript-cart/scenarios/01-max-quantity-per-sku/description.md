Cap quantity per SKU at 99

Warehouse can't fulfil more than 99 units of a single SKU in one order and those orders were being
cancelled by hand. `addItem` now throws a RangeError when the merged quantity would go over the
cap, so the storefront can show the error before checkout. Test added.
