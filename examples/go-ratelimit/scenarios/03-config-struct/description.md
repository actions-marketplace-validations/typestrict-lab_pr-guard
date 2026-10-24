New takes a Config instead of positional arguments

`New(5, 10)` was easy to misread (which one is the burst?) and there was no way to inject a clock
outside the package. New now takes a `Config` with named fields and an optional Clock. All callers
need to switch to `New(Config{Rate: 5, Burst: 10})`; README updated.
