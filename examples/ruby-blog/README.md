# blog

A tiny JSON blog API on Sinatra. Posts live in a JSON file (`BLOG_DATA`).

- `GET /posts` lists published posts, newest first
- `GET /posts/:id` shows one published post
- `POST /admin/posts` and `DELETE /admin/posts/:id` need
  `Authorization: Bearer $BLOG_ADMIN_TOKEN`

Run the tests with `ruby -Itest test/app_test.rb`.
