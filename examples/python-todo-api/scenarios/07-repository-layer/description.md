Move SQL into a TodoRepository

All queries now live in `todo_api/repository.py`, scoped to the current owner, and the views only
deal with HTTP. No behavior change: same endpoints, same status codes, same JSON. The UPDATE and
DELETE statements now also filter by owner, which the views already guaranteed through `_owned`.

Added a unit test for the repository.
