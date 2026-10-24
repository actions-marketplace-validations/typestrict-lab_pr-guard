require "sinatra/base"
require "json"
require "securerandom"
require_relative "lib/post_store"

class BlogApp < Sinatra::Base
  set :store, PostStore.new(ENV.fetch("BLOG_DATA", "posts.json"))
  set :admin_token, ENV.fetch("BLOG_ADMIN_TOKEN", nil)

  helpers do
    def store
      settings.store
    end

    def require_admin!
      token = request.env["HTTP_AUTHORIZATION"].to_s.delete_prefix("Bearer ")
      expected = settings.admin_token
      halt 401, { error: "unauthorized" }.to_json unless expected && Rack::Utils.secure_compare(token, expected)
    end

    def json_body
      JSON.parse(request.body.read)
    rescue JSON::ParserError
      halt 400, { error: "invalid JSON" }.to_json
    end
  end

  before { content_type :json }

  get "/posts" do
    store.published.map(&:to_h).to_json
  end

  get "/posts/:id" do
    post = store.find(params[:id])
    halt 404, { error: "not found" }.to_json unless post&.published?
    post.to_h.merge(body: post.body).to_json
  end

  post "/admin/posts" do
    require_admin!
    attrs = json_body
    halt 422, { error: "title and body are required" }.to_json if attrs["title"].to_s.empty? || attrs["body"].to_s.empty?
    post = Post.new(id: SecureRandom.hex(4), title: attrs["title"], body: attrs["body"],
                    author: attrs.fetch("author", "admin"), published_at: attrs["publish"] ? Time.now : nil)
    status 201
    store.save(post).to_h.to_json
  end

  delete "/admin/posts/:id" do
    require_admin!
    halt 404, { error: "not found" }.to_json unless store.delete(params[:id])
    status 204
    ""
  end
end
