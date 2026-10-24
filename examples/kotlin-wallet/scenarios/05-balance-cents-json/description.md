Serialize balance as balance_cents

The mobile team displayed `balance` as dollars twice this quarter. Naming the JSON field
`balance_cents` makes the unit explicit. Added a serialization test.
