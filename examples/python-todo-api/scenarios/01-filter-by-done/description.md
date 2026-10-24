Add ?done= filter to GET /todos

The mobile client wants to show open and finished items in separate tabs. `GET /todos?done=false`
now returns only open items and `?done=true` only finished ones. Any other value is a 400 so typos
do not silently return everything. Pagination works the same as before.

Added tests for both filters and the invalid value.
