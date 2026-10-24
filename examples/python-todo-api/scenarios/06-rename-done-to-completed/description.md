Rename done to completed in the todo JSON

The new mobile app and the analytics export both call this field `completed`, so the API now
uses the same name in responses and in PATCH bodies. Tests and README updated.
