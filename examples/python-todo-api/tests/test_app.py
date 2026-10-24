import pytest

from todo_api import create_app

TOKENS = {"alice": "alice-token", "bob": "bob-token"}


@pytest.fixture
def client(tmp_path):
    app = create_app({"DATABASE": str(tmp_path / "test.sqlite3"), "API_TOKENS": TOKENS, "TESTING": True})
    return app.test_client()


def auth(user):
    return {"Authorization": f"Bearer {TOKENS[user]}"}


def test_requires_token(client):
    assert client.get("/todos").status_code == 401


def test_create_and_list(client):
    resp = client.post("/todos", json={"title": "Buy milk"}, headers=auth("alice"))
    assert resp.status_code == 201
    assert resp.get_json()["done"] is False
    titles = [t["title"] for t in client.get("/todos", headers=auth("alice")).get_json()]
    assert titles == ["Buy milk"]


def test_rejects_empty_title(client):
    assert client.post("/todos", json={"title": "  "}, headers=auth("alice")).status_code == 400


def test_users_only_see_their_todos(client):
    todo = client.post("/todos", json={"title": "Secret"}, headers=auth("alice")).get_json()
    assert client.get("/todos", headers=auth("bob")).get_json() == []
    resp = client.patch(f"/todos/{todo['id']}", json={"done": True}, headers=auth("bob"))
    assert resp.status_code == 404


def test_mark_done(client):
    todo = client.post("/todos", json={"title": "Ship it"}, headers=auth("alice")).get_json()
    resp = client.patch(f"/todos/{todo['id']}", json={"done": True}, headers=auth("alice"))
    assert resp.get_json()["done"] is True
