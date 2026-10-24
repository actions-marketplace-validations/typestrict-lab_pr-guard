Accept thousands separators in toCents

The admin price import sends values like "1,299.99" and toCents rejected them. Simplified the
parser: strip commas and let parseFloat do the work.
