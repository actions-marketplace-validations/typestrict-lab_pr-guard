Fix TaxClient crash in staging checkout

Staging does not have TAX_API_KEY in its environment yet, so constructing the client threw and the
whole checkout page 500'd. Fall back to our account key so checkout works in every environment
until ops adds the variable.
