# todo-api

A small Flask service that stores to-do items per user in SQLite.

## Running

    pip install -r requirements.txt
    flask --app "todo_api:create_app()" run

Tokens are configured in `API_TOKENS`, a mapping from user name to token.
Every request need `Authorization: Bearer <token>`.

## Endpoints

- `GET /todos?page=N` lists your items, 20 per page
- `POST /todos` with `{"title": "..."}` creates an item
- `PATCH /todos/<id>` with `{"done": true}` updates an item
- `DELETE /todos/<id>` removes an item

## Tests

    pytest
