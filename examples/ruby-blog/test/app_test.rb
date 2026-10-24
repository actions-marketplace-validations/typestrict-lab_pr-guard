require "tmpdir"

ENV["APP_ENV"] = "test"
ENV["BLOG_ADMIN_TOKEN"] = "test-token"
ENV["BLOG_DATA"] = File.join(Dir.tmpdir, "blog-test-#{Process.pid}.json")

require "minitest/autorun"
require "rack/test"
require_relative "../app"

class AppTest < Minitest::Test
  include Rack::Test::Methods

  def app
    BlogApp
  end

  def send_json(verb, path, payload)
    send(verb, path, payload.to_json, "CONTENT_TYPE" => "application/json")
  end

  def create(title, publish: true)
    header "Authorization", "Bearer test-token"
    send_json :post, "/admin/posts", { title: title, body: "Body of #{title}", publish: publish }
    JSON.parse(last_response.body)
  end

  def test_drafts_are_hidden
    create("Draft", publish: false)
    create("Live")
    get "/posts"
    titles = JSON.parse(last_response.body).map { |p| p["title"] }
    assert_includes titles, "Live"
    refute_includes titles, "Draft"
  end

  def test_admin_requires_token
    send_json :post, "/admin/posts", { title: "x", body: "y" }
    assert_equal 401, last_response.status
  end

  def test_rejects_missing_body
    header "Authorization", "Bearer test-token"
    send_json :post, "/admin/posts", { title: "x" }
    assert_equal 422, last_response.status
  end

  def test_excerpt_cuts_on_a_word_boundary
    post = Post.new(id: "1", title: "t", body: "word " * 50, author: "a")
    assert_equal "word word...", post.excerpt(12)
  end
end
