Support nested profiles like eu/prod

We now deploy per region and want `config/eu/prod.toml` next to `config/us/prod.toml`. Profile names
may contain `/` to select a subdirectory. Replaced the traversal test with one for nested profiles.
