require "time"

class Post
  attr_reader :id, :title, :body, :author, :published_at

  def initialize(id:, title:, body:, author:, published_at: nil)
    @id = id
    @title = title
    @body = body
    @author = author
    @published_at = published_at
  end

  def published?(now = Time.now)
    !published_at.nil? && published_at <= now
  end

  def excerpt(length = 140)
    return body if body.length <= length

    body[0, length].sub(/\s+\S*\z/, "") + "..."
  end

  def to_h
    { id: id, title: title, author: author, published_at: published_at&.iso8601, excerpt: excerpt }
  end
end
