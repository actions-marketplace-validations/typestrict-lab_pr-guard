Paginate GET /posts

The front page got slow with 300 posts. `GET /posts?page=N` now returns 10 posts per page,
starting at page 1, and the X-Total-Pages header tells the frontend how many page links to render.
