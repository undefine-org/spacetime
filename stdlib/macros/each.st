/// Each Macro
///
/// Iterates over a data source and renders templates for each item.
/// Supports enter/exit animations and keyed updates.
///
/// @example basic - with template invocation
/// .product-list {
///   @each($products as $p) {
///     &product-card($p);
///   }
/// }
///
/// @example with key for diffing
/// .todo-list {
///   @each($todos as $t, key: $t.id) {
///     &todo-item($t);
///
///     :entering {
///       opacity: 0 -> 1
///       translate-x: -20px -> 0
///     }
///
///     :exiting {
///       opacity: 1 -> 0
///       translate-x: 0 -> 20px
///     }
///   }
/// }
///
/// @example multiple templates
/// .dashboard {
///   @each($widgets as $w) {
///     &widget-header($w.title);
///     &widget-content($w);
///     &widget-footer($w.meta);
///   }
/// }

%macro each {
  %order 600

  %form {
    @each($source:binding as $item:binding, key: $key:expr? = undefined) {
      $invocations:template_invocation+
      (:entering { $_enterAnim:keyframes })?
      (:exiting { $_exitAnim:keyframes })?
      (:move { $_moveAnim:keyframes })?
    }
  }

  %iterates $source {
    // Built-in loop variables bound to $item
    item: $item       // current item (named)
    index: $_index    // 0-based index
    first: $_first    // true for first item
    last: $_last      // true for last item
    count: $_count    // total item count
    key: $key         // unique key for diffing
  }

  %binds {
    each-with-templates(
      &self,
      source: $source,
      item: $item,
      invocations: $invocations,
      key: $key
    ) -> { $items, $update }
  }

  %transitions {
    enter: $enterAnim ?? {
      opacity: 0 -> 1
    }

    exit: $exitAnim ?? {
      opacity: 1 -> 0
    }

    move: $moveAnim ?? {
      // Default: smooth position transitions
    }

    duration: 300ms
    easing: ease-out
  }
}


/// Each Macro (with filter)
///
/// Filters rendered items based on a reactive expression.
/// Non-matching items receive `display: none` and are reactively re-filtered
/// when state variables in the filter expression change.
///
/// @example filter by category
/// .events-list {
///   @each($events as $e, when $filterCategory == "all" || $e.typeCode == $filterCategory) {
///     &event-row($e);
///   }
/// }

%macro each-filtered {
  %order 599

  %form {
    @each($source:binding as $item:binding, when $filter:expr) {
      $invocations:template_invocation+
      (:entering { $_enterAnim:keyframes })?
      (:exiting { $_exitAnim:keyframes })?
      (:move { $_moveAnim:keyframes })?
    }
  }

  %form {
    @each($source:binding as $item:binding, key: $key:expr? = undefined, when $filter:expr) {
      $invocations:template_invocation+
      (:entering { $_enterAnim:keyframes })?
      (:exiting { $_exitAnim:keyframes })?
      (:move { $_moveAnim:keyframes })?
    }
  }

  %iterates $source {
    item: $item
    index: $_index
    first: $_first
    last: $_last
    count: $_count
    key: $key
  }

  %binds {
    each-with-templates(
      &self,
      source: $source,
      item: $item,
      invocations: $invocations,
      key: $key,
      filter: $filter
    ) -> { $items, $update }
  }

  %transitions {
    enter: $enterAnim ?? {
      opacity: 0 -> 1
    }

    exit: $exitAnim ?? {
      opacity: 1 -> 0
    }

    move: $moveAnim ?? {
      // Default: smooth position transitions
    }

    duration: 300ms
    easing: ease-out
  }
}


/// For Macro
///
/// Simple numeric iteration.
///
/// @example
/// .star-rating {
///   @for($i in 1..5) {
///     > star-icon {
///       [data-index]: $i
///       [class]: $i <= $rating ? "filled" : "empty"
///     }
///   }
/// }

%macro for-range {

  %form {
    @for($_var:ident in $start:number .. $end:number) {
      $_template:template
    }
  }

  %derives {
    $items: range($start, $end)
  }

  %iterates $items {
    item: $var
    index: $_index
    first: $_first
    last: $_last
    count: $_count
  }

  %template {
    $template
  }
}


/// Repeat Macro
///
/// Simple count-based repetition.
///
/// @example
/// .loading-skeleton {
///   @repeat(5) {
///     > .skeleton-line
///   }
/// }

%macro repeat {

  %form {
    @repeat($count:number) {
      $_template:template
    }
  }

  %derives {
    $items: range(0, $count)
  }

  %iterates $items {
    index: $_index
    first: $_first
    last: $_last
    count: $_count
  }

  %template {
    $template
  }
}


/// If/Else Macro
///
/// Conditional rendering.
///
/// !! NOT IMPLEMENTED (BUG-362). This macro declares a `%form` and a `%when`
/// clause but has NO `%emit` block and no primitive behind it, so nothing
/// renders — `@if($x) { … }` fails with `E0956: unknown primitive:
/// conditional`, and even a literal `@if(true)` produces no output. Inside a
/// `@template`'s markup the source leaks as page TEXT instead (W0712 /
/// BUG-358).
///
/// It looks usable because a conditional that renders NOTHING satisfies every
/// `should not_exist` assertion, so the false branch always appears correct.
/// There are zero usages in stdlib, demos and examples.
///
/// `@each` below is the model a real implementation must follow: a primitive
/// with an `%emit` block that produces DOM and subscribes to its source. Note
/// that `%when` is evaluated at COMPILE time against captured macro bindings,
/// so it cannot see a runtime signal — it is not the mechanism this needs.
///
/// @example — the INTENDED surface, once implemented
/// .user-status {
///   @if($user.isOnline) {
///     > .online-indicator { "Online" }
///   } @else {
///     > .offline-indicator { "Offline" }
///   }
/// }

%macro conditional {

  %form {
    @if($_condition:expr) {
      $_thenTemplate:template
    }
  }

  %when $condition {
    %template {
      $thenTemplate
    }
  }
}


%macro conditional-else {

  %form {
    @else {
      $_elseTemplate:template
    }
  }

  // @else must follow @if - compiler handles pairing
  %when !$_previousCondition {
    %template {
      $elseTemplate
    }
  }
}


/// Each Macro (inline HTML body) — PLAN-023 W4
///
/// Renders raw inline HTML once per item, instead of invoking a @template.
/// `$.field` / `$item.field` interpolate the current item; `$index` the position.
///
/// @example
/// .todo-list {
///   @each($todos as $t) {
///     <li class="todo">`$t.title`</li>
///   }
/// }
///
/// The HTML-body form is selected over the template-invocation form because the
/// `$invocations:template_invocation+` capture rejects a non-invocation (HTML)
/// body (PLAN-023 W4 form discrimination), so this `$body:html_block` form wins.
%macro each-inline {
  %order 598

  %form {
    @each($source:binding as $item:binding) {
      $body:html_block
    }
  }

  %iterates $source {
    item: $item
    index: $_index
  }

  %binds {
    each-inline(
      &self,
      source: $source,
      item: $item,
      html: $body
    ) -> { $items }
  }
}

/// Each-Inline with reactive `when` filter (FEAT-094). Same inline-HTML body as
/// `each-inline`, plus a `when $filter:expr` clause: rows whose filter is false
/// get `display: none`, and the list re-filters whenever a state variable in the
/// expression changes (live search / category filter, zero custom JS).
///
/// @example live search
///   .grid {
///     @each($items as $i, when $q == "" || $i.name.indexOf($q) >= 0) {
///       <article class="card">`$i.name`</article>
///     }
///   }
%macro each-inline-filtered {
  %order 597

  %form {
    @each($source:binding as $item:binding, when $filter:expr) {
      $body:html_block
    }
  }

  %iterates $source {
    item: $item
    index: $_index
  }

  %binds {
    each-inline(
      &self,
      source: $source,
      item: $item,
      html: $body,
      filter: $filter
    ) -> { $items }
  }
}
