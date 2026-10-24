from flask import Flask, abort, g, jsonify, request

from . import db
from .auth import require_token

PAGE_SIZE = 20


def to_json(row):
    return {
        "id": row["id"],
        "title": row["title"],
        "done": bool(row["done"]),
        "created_at": row["created_at"],
    }


def create_app(config=None):
    app = Flask(__name__)
    app.config.from_mapping(DATABASE="todos.sqlite3", API_TOKENS={})
    if config:
        app.config.update(config)
    app.teardown_appcontext(db.close_db)
    with app.app_context():
        db.init_db()

    @app.get("/todos")
    @require_token
    def list_todos():
        page = max(request.args.get("page", 1, type=int), 1)
        offset = (page - 1) * PAGE_SIZE
        rows = db.get_db().execute(
            "SELECT * FROM todos WHERE owner = ? ORDER BY id LIMIT ? OFFSET ?",
            (g.user, PAGE_SIZE, offset),
        ).fetchall()
        return jsonify([to_json(r) for r in rows])

    @app.post("/todos")
    @require_token
    def create_todo():
        payload = request.get_json(silent=True) or {}
        title = str(payload.get("title", "")).strip()
        if not title or len(title) > 200:
            return jsonify(error="title must have 1 to 200 characters"), 400
        conn = db.get_db()
        cur = conn.execute("INSERT INTO todos (owner, title) VALUES (?, ?)", (g.user, title))
        conn.commit()
        row = conn.execute("SELECT * FROM todos WHERE id = ?", (cur.lastrowid,)).fetchone()
        return jsonify(to_json(row)), 201

    @app.patch("/todos/<int:todo_id>")
    @require_token
    def update_todo(todo_id):
        row = _owned(todo_id)
        payload = request.get_json(silent=True) or {}
        done = bool(payload.get("done", row["done"]))
        conn = db.get_db()
        conn.execute("UPDATE todos SET done = ? WHERE id = ?", (int(done), todo_id))
        conn.commit()
        return jsonify(to_json(_owned(todo_id)))

    @app.delete("/todos/<int:todo_id>")
    @require_token
    def delete_todo(todo_id):
        _owned(todo_id)
        conn = db.get_db()
        conn.execute("DELETE FROM todos WHERE id = ?", (todo_id,))
        conn.commit()
        return "", 204

    return app


def _owned(todo_id):
    row = db.get_db().execute(
        "SELECT * FROM todos WHERE id = ? AND owner = ?", (todo_id, g.user)
    ).fetchone()
    if row is None:
        abort(404)
    return row
