// GH-37: `@data signal $cursor(x number, y number) to $host` uses BARE param
// names. param_list only accepts `$binding` / `&element` (typed as `$x number`).
// Issue claims: compiles clean and emits `callParams = []` with no diagnostic.
@import "stdlib"

@host $api : http("https://example.com")

// The issue's shape: bare `x number, y number` (NOT `$x number`).
@data signal $cursor(x number, y number) to $api {
  send emit "cursor" { x: $x, y: $y }
  receive to Cursor {
    _ => Cursor($.body)
  }
}

// Control: the CANONICAL typed form `$x number`.
@data signal $add($text string) to $api {
  send POST "/add" { text: $text }
  receive to AddResult {
    200 => Ok($.body)
    _   => Err($.statusText)
  }
}

.probe { color: red; }
