require "json"
require_relative "post"

# Keeps posts in a JSON file; good enough for a single-author blog.
class PostStore
  def initialize(path)
    @path = path
    @posts = File.exist?(path) ? read : {}
  end

  def all
    @posts.values.sort_by { |p| p.published_at || Time.at(0) }.reverse
  end

  def published(now = Time.now)
    all.select { |p| p.published?(now) }
  end

  def find(id)
    @posts[id]
  end

  def save(post)
    @posts[post.id] = post
    write
    post
  end

  def delete(id)
    @posts.delete(id).tap { write }
  end

  private

  def read
    JSON.parse(File.read(@path)).to_h do |attrs|
      published_at = attrs["published_at"] && Time.iso8601(attrs["published_at"])
      post = Post.new(id: attrs["id"], title: attrs["title"], body: attrs["body"],
                      author: attrs["author"], published_at: published_at)
      [post.id, post]
    end
  end

  def write
    rows = @posts.values.map { |p| p.to_h.except(:excerpt).merge(body: p.body) }
    File.write(@path, JSON.pretty_generate(rows))
  end
end
