Introduce ProfileName and move the serde duration adapter

Profile validation now lives in a `ProfileName` newtype (with its own unit test) so the coming
`list_profiles` function can reuse it, and the serde adapter for durations moves next to the
parser. No behavior change: same accepted names, same errors.
