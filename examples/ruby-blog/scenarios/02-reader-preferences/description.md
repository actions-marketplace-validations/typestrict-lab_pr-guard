Remember the reader's sort order in a cookie

Readers can now choose to read oldest first. `PUT /preferences` with `{"order": "oldest"}` stores
the choice in a cookie and `GET /posts` honours it. No accounts needed.
