Add title search to the todo list

Users with a lot of items asked for a search box. `GET /todos?q=milk` now returns only the items
whose title contains the term. Without `q` the endpoint behaves exactly as before.
