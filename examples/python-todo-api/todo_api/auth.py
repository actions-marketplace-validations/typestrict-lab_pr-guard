import hmac
from functools import wraps

from flask import current_app, g, jsonify, request


def require_token(view):
    """Resolve the bearer token to a user name and store it in g.user."""

    @wraps(view)
    def wrapped(*args, **kwargs):
        header = request.headers.get("Authorization", "")
        scheme, _, token = header.partition(" ")
        if scheme.lower() != "bearer" or not token:
            return jsonify(error="missing bearer token"), 401
        for user, expected in current_app.config["API_TOKENS"].items():
            if hmac.compare_digest(token, expected):
                g.user = user
                return view(*args, **kwargs)
        return jsonify(error="invalid token"), 401

    return wrapped
