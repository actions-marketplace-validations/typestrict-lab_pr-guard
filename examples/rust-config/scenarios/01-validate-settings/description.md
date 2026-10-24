Reject workers = 0 and a zero request timeout

A typo in staging (`workers = 0`) started a server that accepted connections and never answered
them. `load` now validates the parsed config and returns `LoadError::Invalid` with the reason.
LoadError is already non_exhaustive, so adding the variant is not a breaking change.
