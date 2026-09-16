defmodule SpacetimeLvWeb.Router do
  use SpacetimeLvWeb, :router

  pipeline :browser do
    plug :accepts, ["html"]
    plug :fetch_session
    plug :fetch_live_flash
    plug :put_root_layout, html: {SpacetimeLvWeb.Layouts, :root}
    plug :protect_from_forgery
    plug :put_secure_browser_headers
  end

  pipeline :api do
    plug :accepts, ["json"]
  end

  scope "/", SpacetimeLvWeb do
    pipe_through :browser

    live "/", CounterLive
    live "/board", BoardLive
    live "/signup", SignupLive
    live "/signup-field", SignupFieldLive
  end

  # Other scopes may use custom stacks.
  # scope "/api", SpacetimeLvWeb do
  #   pipe_through :api
  # end
end
