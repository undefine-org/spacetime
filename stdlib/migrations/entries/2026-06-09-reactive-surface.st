/// FEAT-072 wave (2026-06-09): the paren-macro reactive surface — `@bind(…)`,
/// `@show(…)`, `@input(…)` — retired in favor of the `:` / `<-` reactive
/// surface (class toggles `.shown: $v;`, arrow bindings `text <- $x;`,
/// `@on` handlers).
///
/// THE CAPSULE (PLAN-079): this entry owns EVERYTHING about the retired
/// syntax — the old definitions VERBATIM below (grammar + `%binds` semantics,
/// recovered from `stdlib/macros/data.st` @ 75ce7975^), the mechanical
/// rewrites, and the manual guidance. Stdlib proper carries only current
/// syntax; deleting this file deletes the old syntax from existence.
///
/// While a project's `@version` predates this wave: mechanical shapes are
/// auto-rewritten in memory (W0715) and persisted by `spacetime migrate` or
/// the dev-dock pill; `@show`/`@input` still compile via the embedded
/// definitions' `%binds` (the bind-* primitives are retained), with a W0715
/// carrying the %hint. Once `@version` reaches 2026-06-09, any lingering use
/// is E0911.
%migration reactive-surface {
  %date 2026-06-09
  %docs "FEAT-072: the paren-macro reactive surface (@bind/@show/@input) became `:` / `<-`."

  // ── Retired definitions, verbatim (stdlib/macros/data.st @ 75ce7975^) ──

  %macro bind {
    %form {
      @bind(
        text: $text:expr? = undefined,
        attr: $attr:string? = "",
        value: $value:expr? = undefined,
        class: $class:string? = "",
        when: $condition:expr? = undefined,
        style: $styleProp:string? = "",
        visible: $visible:expr? = undefined
      )
    }

    // Bind text content
    %if $text {
      %binds {
        bind-text(&self, value: $text)
      }
    }

    // Bind attribute
    %if $attr {
      %binds {
        bind-attr(&self, attr: $attr, value: $value)
      }
    }

    // Bind class
    %if $class {
      %binds {
        bind-class(&self, className: $class, condition: $condition)
      }
    }

    // Bind style
    %if $styleProp {
      %binds {
        bind-style(&self, property: $styleProp, value: $value)
      }
    }

    // Bind visibility
    %if $visible {
      %binds {
        bind-visible(&self, condition: $visible)
      }
    }
  }

  %macro show {
    %form {
      @show(
        when: $condition:expr,
        mode: $mode:string = "display"
      )
    }

    %binds {
      bind-visible(&self, condition: $condition, mode: $mode)
    }
  }

  %macro input-bind {
    %form {
      @input(
        bind: $value:expr,
        event: $event:string = "input",
        debounce: $debounce:time = 0ms
      )
    }

    %binds {
      bind-input(&self, value: $value, event: $event, debounce: $debounce) -> { $value }
    }
  }

  // ── Mechanical rewrites (per shape; the shim + `spacetime migrate`) ──

  %rewrite bind-text {
    %match {
      @bind(text: $x:expr)
    }
    %into {
      text <- `$x`;
    }
  }

  %rewrite bind-attr {
    %match {
      @bind(attr: $a:string, value: $v:expr)
    }
    %into {
      `$a` <- `$v`;
    }
  }

  %rewrite bind-class {
    %match {
      @bind(class: $c:string, when: $w:expr)
    }
    %into {
      .`$c`: `$w`;
    }
  }

  %rewrite bind-style {
    %match {
      @bind(style: $s:string, value: $v:expr)
    }
    %into {
      `$s`: `$v`;
    }
  }

  %rewrite bind-visible {
    %match {
      @bind(visible: $v:expr)
    }
    %into {
      .shown: `$v`;
    }
  }

  // ── Manual guidance ──

  %hint for @bind "One property per statement in the new surface: split multi-property @bind calls — `text <- $x;`, `name <- $v;`, `.class: $cond;`, `prop: $v;`. See docs/MIGRATION.md."
  %hint for @show "Replace with a reactive class toggle — e.g. `.hidden: !$condition;` — plus a `.hidden { display: none }` rule (or `visibility` for mode: \"visibility\"). See docs/MIGRATION.md."
  %hint for @input "Replace with a two-way binding: `value <- $signal;` + `@on &.input { $signal <- &el.value; }` (use @on change for event: \"change\"). See docs/MIGRATION.md."
}
