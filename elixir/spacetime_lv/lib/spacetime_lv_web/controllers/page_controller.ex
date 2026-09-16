defmodule SpacetimeLvWeb.PageController do
  use SpacetimeLvWeb, :controller

  def home(conn, _params) do
    render(conn, :home)
  end
end
