Extract PostSerializer

Post knew about its API shape and PostStore duplicated the storage shape. Both now go through
`PostSerializer` (summary, full, storage, from_storage), which gets its own tests. The API
responses and the JSON file format are unchanged.
