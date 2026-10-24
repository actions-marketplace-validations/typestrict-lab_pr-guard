Simplify currency conversion

The BigDecimal chain in `convert` was hard to read for a one-line multiplication. Use plain
arithmetic on the rate instead.
