/// Unified @data <kind> surface — source<T> (PLAN-023 W3, FEAT-046)
///
/// One construct, one kind-word discriminator (like @on's event word):
///
///   @data inline $name <type>? : <value>     // static local state
///   @data fetch  $name <type>? : <url>        // remote/async source
///   @data derive $name <type>? : <expr>       // stateless computed (deps from expr)
///   @data fold   $name <type>? : <expr>       // stateful accumulation (deps from expr)
///
/// THE KIND IS THE MACRO: each kind is its own %macro whose %form carries the
/// literal kind-word, so parse disambiguates by specificity (no `%if $src==`
/// string-switch). The binding name is captured as `:binding` (NOT `:ident`) so
/// the `$`-prefixed sigil token is accepted; the leading `$` is stripped for the
/// emitted JS name. Each macro registers a `binding(...)` with a `source_kind`
/// and `source_static` fact that W4 uses to choose SSG-unroll vs hydrate.
///
/// `@computed` (the old keyword) folds into `derive`/`fold`.


/// @data inline — static local state (compile-time constant value).
/// static: true  → W4 may SSG-unroll lists that read it.
%macro data-inline {
  %scope file | selector
  %order 250

  %form {
    @data inline $name:binding $type:typeref? : $value:expr ;
  }

  %binds {
    local-state-impl(name: $name, initial: $value) -> { $state as $name }
  }

  %registers binding($name, type: $type) {
    value: $name
    writable: true
    source_kind: "inline"
    source_static: true
  }
}


/// ERROR SIBLING (BUG-264): a bare NUMBER in the type slot (`@data inline $x 5 : …`)
/// is a near-miss for the `$type:typeref?` optional. The productive `data-inline`
/// form REJECTS it (strict-capture: `:typeref` won't bind a number), so the value
/// would otherwise be silently dropped and the type slot vanish. This sibling —
/// which binds the offending number — reports it with a data-supplied message.
/// Declared AFTER `data-inline`: the productive form wins cleanly on a valid type
/// name, leaving this sibling to fire only when the type slot holds a number.
%macro data-inline-invalid-type {
  %scope file | selector
  %order 250

  %form {
    @data inline $name:binding $type:number : $value:expr ;
  }

  %binds {
    optional-clause-error(
      message: "a bare number is not a valid type",
      hint: "write a type name like `number`, `string`, or `Product[]` in the slot after `$name` — not a value"
    )
  }
}


/// `@data graphql` — a GraphQL query as a data source.
///
/// The `graphql` primitive existed, complete and tested, with NO way to write
/// it: no `%macro` bound it, so the capability shipped in the bundle and could
/// not be reached from a page (PLAN-141 W1.2, from the G1 reachability report).
///
/// It is an ARM of the data-kind family, not a new concept. `graphql` exports
/// `$data`/`$loading`/`$error`/`$refetch` — precisely what `data-source` exports
/// for `@data fetch` — so the author-facing shape is the one they already know:
///
///     @data fetch   $posts : "/api/posts";
///     @data graphql $posts : "/graphql" query: "{ posts { id title } }";
///
/// and `$posts`, `$posts_loading`, `$posts_error`, `$posts_refetch` all mean the
/// same thing in both. That is the "learnable by pattern" rule doing real work:
/// knowing the fetch arm tells you this one.
///
/// Variables are optional and default to `{}` — the same optionality shape the
/// primitive already declares, expressed as an inline optional group rather than
/// a sibling form (W3's `%form` groups, so the two spellings cannot compete).
%macro data-graphql-kind {
  %scope file | selector
  %order 250

  %form {
    @data graphql $name:binding $type:typeref? : $url:expr , query : $query:expr ;
  }

  %binds {
    graphql(
      url: $url,
      query: $query,
      variables: {}
    ) -> {
      $data as $name,
      $loading as ${$name}_loading,
      $error as ${$name}_error,
      $refetch as ${$name}_refetch
    }
  }

  %registers binding($name, type: $type) {
    value: $name
    loading: ${$name}_loading
    error: ${$name}_error
    refetch: ${$name}_refetch
    source_kind: "graphql"
    source_static: false
  }
}

/// @data fetch — remote/async source. static: false (resolves at runtime).
%macro data-fetch-kind {
  %scope file | selector
  %order 250

  // Bodyless: `@data fetch $api : "/url";`
  %form {
    @data fetch $name:binding $type:typeref? : $src:expr ;
  }

  %binds {
    data-source(
      name: $name,
      src: $src,
      default: null,
      refresh: 0
    ) -> {
      $data as $name,
      $loading as ${$name}_loading,
      $error as ${$name}_error,
      $reload as ${$name}_refetch
    }
  }

  %registers binding($name, type: $type) {
    value: $name
    loading: ${$name}_loading
    error: ${$name}_error
    refetch: ${$name}_refetch
    source_kind: "fetch"
    source_static: false
  }
}



/// ERROR SIBLING (BUG-264): a bare NUMBER in the type slot (`@data fetch $x 5 : …`)
/// is a near-miss for the `$type:typeref?` optional on the bodyless `@data fetch`.
/// The productive `data-fetch-kind` form REJECTS it (strict-capture: `:typeref`
/// won't bind a number), so the value would otherwise be silently dropped. This
/// sibling — which binds the offending number — reports it. Declared AFTER
/// `data-fetch-kind`: the productive form wins cleanly on a valid type name.
%macro data-fetch-invalid-type {
  %diagnostic
  %scope file | selector
  %order 250

  %form {
    @data fetch $name:binding $type:number : $src:expr ;
  }

  %binds {
    optional-clause-error(
      message: "a bare number is not a valid type",
      hint: "write a type name like `number`, `string`, or `Product[]` in the slot after `$name` — not a value"
    )
  }
}


/// @data derive — stateless computed. static: AND(deps.static); deps discovered
/// from the value expression at runtime by the derived-signal primitive.
/// Folds in the old @computed keyword.
%macro data-derive {
  %scope file | selector
  %order 250

  %form {
    @data derive $name:binding $type:typeref? : $value:balanced(';') ;
  }

  %binds {
    derived-signal(name: $name, expr: $value, fold: false) -> { $value as $name }
  }

  %registers binding($name, type: $type) {
    value: $name
    computed: true
    source_kind: "derive"
    source_static: false
  }
}



/// ERROR SIBLING (BUG-264): a bare NUMBER in the type slot (`@data derive $x 5 : …`)
/// is a near-miss for the `$type:typeref?` optional. The productive `data-derive`
/// form REJECTS it (strict-capture), so the value would otherwise be silently
/// dropped. This sibling reports it. Declared AFTER `data-derive`.
%macro data-derive-invalid-type {
  %scope file | selector
  %order 250

  %form {
    @data derive $name:binding $type:number : $value:balanced(';') ;
  }

  %binds {
    optional-clause-error(
      message: "a bare number is not a valid type",
      hint: "write a type name like `number`, `string`, or `Product[]` in the slot after `$name` — not a value"
    )
  }
}


/// @data fold — stateful accumulation. static: false. The expr is a reducer
/// `($src, acc, item) -> step`; derived-signal reduces over the source array.
%macro data-fold {
  %scope file | selector
  %order 250

  %form {
    @data fold $name:binding $type:typeref? from $source:binding : $value:balanced(';') ;
  }

  %binds {
    derived-signal(name: $name, expr: $value, fold: true, source: $source, initial: 0) -> { $value as $name }
  }

  %registers binding($name, type: $type) {
    value: $name
    computed: true
    source_kind: "fold"
    source_static: false
  }
}



/// ERROR SIBLING (BUG-264): a bare NUMBER in the type slot (`@data fold $x 5 from …`)
/// is a near-miss for the `$type:typeref?` optional. The productive `data-fold`
/// form REJECTS it (strict-capture), so the value would otherwise be silently
/// dropped. This sibling reports it. Declared AFTER `data-fold`.
%macro data-fold-invalid-type {
  %diagnostic
  %scope file | selector
  %order 250

  %form {
    @data fold $name:binding $type:number from $source:binding : $value:balanced(';') ;
  }

  %binds {
    optional-clause-error(
      message: "a bare number is not a valid type",
      hint: "write a type name like `number`, `string`, or `Product[]` in the slot after `$name` — not a value"
    )
  }
}


/// @data query — array query pipeline (where/sort/limit/map/reduce) over a source
/// signal. The array-query counterpart to scalar `@data derive`; binds the
/// computed-source primitive (filter/sort/limit/transform/reduce). Replaces the
/// legacy `@computed` query DSL. PLAN-023 W5.
///
/// @example
///   @data query $active Product[] from $products {
///     where: $.inStock == true;
///     sort: $.price;
///     dir: asc;
///     limit: 6;
///   }
%macro data-query {
  %scope file | selector
  %order 250

  %form {
    @data query $name:binding $type:typeref? from $source:binding {
      where: $filter:balanced(';')?
      sort: $sortBy:balanced(';')?
      dir: $sortDir:ident?
      limit: $limit:number?
      map: $transform:balanced(';')?
      reduce: $reduce:balanced(';')?
      initial: $initial:balanced(';')?
    }
  }

  %binds {
    computed-source(
      name: $name,
      source: $source,
      filter: $filter,
      sortBy: $sortBy,
      sortDir: $sortDir,
      limit: $limit,
      transform: $transform,
      reduce: $reduce,
      initial: $initial
    ) -> { $value as $name }
  }

  %registers binding($name, type: $type) {
    value: $name
    computed: true
    source_kind: "query"
    source_static: false
  }
}



/// ERROR SIBLING (BUG-264): a bare NUMBER in the type slot (`@data query $x 5 from …`)
/// is a near-miss for the `$type:typeref?` optional on `@data query`. The productive
/// `data-query` form REJECTS it (strict-capture), so the value would otherwise be
/// silently dropped. This sibling — which binds the offending number and captures
/// the body as opaque balanced text — reports it. Declared AFTER `data-query`: the
/// productive form wins cleanly on a valid type name.
%macro data-query-invalid-type {
  %diagnostic
  %scope file | selector
  %order 250

  %form {
    @data query $name:binding $type:number from $source:binding {
      where: $filter:balanced(';')?
      sort: $sortBy:balanced(';')?
      dir: $sortDir:ident?
      limit: $limit:number?
      map: $transform:balanced(';')?
      reduce: $reduce:balanced(';')?
      initial: $initial:balanced(';')?
    }
  }

  %binds {
    optional-clause-error(
      message: "a bare number is not a valid type",
      hint: "write a type name like `number`, `string`, or `Product[]` in the slot after `$name` — not a value"
    )
  }
}


/// @data fetch (with options body) — remote source with refresh/initial.
/// Companion to the bodyless `@data fetch` (mirrors @each / @each-filtered: a
/// distinct %macro per body shape, since a %macro carries one %form). PLAN-023 W5.
///
/// @example
///   @data fetch $products Product[] : "/api/products" {
///     refresh: 5m;
///     initial: [];
///   }
%macro data-fetch-opts {
  %scope file | selector
  %order 249

  %form {
    @data fetch $name:binding $type:typeref? : $src:expr {
      refresh: $refresh:duration?
      initial: $initial:balanced(';')?
    }
  }

  %binds {
    data-source(
      name: $name,
      src: $src,
      default: $initial,
      refresh: $refresh
    ) -> {
      $data as $name,
      $loading as ${$name}_loading,
      $error as ${$name}_error,
      $reload as ${$name}_refetch
    }
  }

  %registers binding($name, type: $type) {
    value: $name
    loading: ${$name}_loading
    error: ${$name}_error
    refetch: ${$name}_refetch
    source_kind: "fetch"
    source_static: false
  }
}

/// ERROR SIBLING (BUG-264): a `refresh:` value that is a NEAR-MISS for a
/// duration (`refresh: 5`, `5px`, `foo`, `5 ms` — anything that is neither a
/// duration literal nor a `$signal`) is refused by the productive
/// `data-fetch-opts` form (strict-capture: the `:duration` capture rejects any
/// value that is not a duration and not a `$`-reference). This sibling — which
/// receives the refresh text — reports it instead of letting it silently drop.
/// Declared AFTER `data-fetch-opts`: both forms have equal specificity, so the
/// tie breaks by declaration order and the productive form wins cleanly on
/// valid `refresh: 5s`, leaving this sibling to fire only when the productive
/// form refused a near-miss.
%macro data-fetch-opts-invalid-refresh {
  %diagnostic
  %scope file | selector
  %order 249

  %form {
    @data fetch $name:binding $type:typeref? : $src:expr {
      refresh: $refresh:expr
    }
  }

  %binds {
    optional-clause-error(
      message: "`refresh:` must be a duration or a signal, not a bare number",
      hint: "write `refresh: 5s` — refresh takes a duration (e.g. `5s`, `1m`) or a `$signal`",
      value: $refresh
    )
  }
}


/// ERROR SIBLING (BUG-264): a bare NUMBER in the type slot (`@data fetch $x 5 : … { }`)
/// is a near-miss for the `$type:typeref?` optional on the BODY `@data fetch`
/// (`data-fetch-opts`). The productive form REJECTS it (strict-capture), so the
/// value would otherwise be silently dropped. This sibling reports it. Declared
/// AFTER `data-fetch-opts` (and its refresh sibling): the productive form wins
/// cleanly on a valid type name.
%macro data-fetch-opts-invalid-type {
  %diagnostic
  %scope file | selector
  %order 249

  %form {
    @data fetch $name:binding $type:number : $src:expr {
      refresh: $refresh:duration?
      initial: $initial:balanced(';')?
    }
  }

  %binds {
    optional-clause-error(
      message: "a bare number is not a valid type",
      hint: "write a type name like `number`, `string`, or `Product[]` in the slot after `$name` — not a value"
    )
  }
}


/// @data registry — the compiler's own primitive/macro registry as a STATIC
/// data source (FEAT-084). Each row is `{ name, kind, doc, params[], exports[] }`
/// (the FEAT-083-enriched introspection summary). These %macros only RECOGNISE
/// the surface syntax; the pipeline (`src/compiler.rs::inject_registry_data`)
/// rewrites a matched `@data registry` into a `@data inline` whose value is the
/// registry JSON synthesised from the in-memory MetaRegistry — so it flows
/// through the IDENTICAL inline path (static local-state + `@each` SSG-unroll).
/// The self-documenting docs gallery reads this; add a %primitive and a card
/// appears next build, zero page edits.
///
///   @data registry $features ;                 // all documentable entries
///   @data registry $tests from kind "test" ;   // a filtered slice
%macro data-registry-all {
  %pipeline-consumed
  %scope file | selector
  %order 250
  %form {
    @data registry $name:binding ;
  }
}

%macro data-registry-filtered {
  %pipeline-consumed
  %scope file | selector
  %order 250
  %form {
    @data registry $name:binding from $filterKey:ident $filterVal:string ;
  }
}

/// @data dispatch — the compiler's own macro DISPATCH catalog as a STATIC data
/// source (FEAT-089). Each row is `{ directive, name, signature, tokens[], doc,
/// kind, scopes[], binds[], overloads[] }` for one %form-carrying macro — the
/// exact forms the predicate-dispatch scorer reads. `tokens` is the structured
/// signature (text + role) so a UI can chip-style each piece. These %macros only
/// RECOGNISE the surface; `src/compiler.rs::inject_dispatch_data` rewrites a match
/// into a `@data inline` whose value is the catalog JSON, so it flows through the
/// IDENTICAL inline path (static local-state + `@each`). The macro reference reads
/// this; add a %macro and a card appears next build.
///
///   @data dispatch $macros ;                  // every dispatchable macro
///   @data dispatch $data from kind "data" ;    // a filtered slice
%macro data-dispatch-all {
  %pipeline-consumed
  %scope file | selector
  %order 250
  %form {
    @data dispatch $name:binding ;
  }
}

%macro data-dispatch-filtered {
  %pipeline-consumed
  %scope file | selector
  %order 250
  %form {
    @data dispatch $name:binding from $filterKey:ident $filterVal:string ;
  }
}


/// @data forms — the build's `@form` DECLARATION catalog as a STATIC data
/// source (PLAN-135 W4). Each row is `{ name, kind, params, body, doc,
/// source }` for one `@form <kind> --name { … }` declaration the build can
/// see: the page, its `@import`s, the `_prelude.st` project overlay, and the
/// stdlib preset library. `params` renders the declaration's parameter list
/// (`$pad = 8px`), `body` the declared chunk's own text, `doc` the `///`
/// block above the declaration, `source` its file. These %macros only
/// RECOGNISE the surface; `src/compiler.rs::inject_forms_data` rewrites a
/// match into a `@data inline` whose value is the catalog JSON, so it flows
/// through the IDENTICAL inline path (static local-state + `@each`). A brand
/// catalog page reads this; add a declaration to `_prelude.st` and a card
/// appears next build, zero page edits.
///
///   @data forms $forms ;                     // every visible declaration
///   @data forms $curves from kind "easing" ;  // a filtered slice

/// @data declarations — every DECLARATION of a given registry entity, as rows.
///
/// The general form of `@data forms`. `%registers <entity>` is the metasystem's
/// generic rail (driver, binding, form, host, template, type, …), so a catalog
/// page can enumerate ANY of them through one surface instead of a sibling
/// macro per kind. `@data forms $x ;` is exactly `@data declarations $x from
/// form ;` — kept as sugar because it is the common case.
///
/// Rows carry `name`, `kind`, `params`, `body`, `doc`, `source`, `origin`, and
/// `fields` — the `/// key: value` lines of the declaration's own doc comment,
/// parsed into a map, so a pattern's `scenario` / `school` / `useWhen` are data
/// a page can group and filter by.
///
///   @data declarations $patterns from template ;
///   @data declarations $curves   from easing ;

%macro data-declarations {
  %pipeline-consumed
  %scope file | selector
  %order 250
  %form {
    @data declarations $name:binding from $entity:ident ;
  }
}

%macro data-forms-all {
  %pipeline-consumed
  %scope file | selector
  %order 250
  %form {
    @data forms $name:binding ;
  }
}

%macro data-forms-filtered {
  %pipeline-consumed
  %scope file | selector
  %order 250
  %form {
    @data forms $name:binding from $filterKey:ident $filterVal:string ;
  }
}


/// @data collection — a MUTABLE remote array with optimistic CRUD (FEAT-075).
///
///   @data collection $rows Product[] from "/data/products.json" via dev-ws {
///     initial: [];
///   }
///
/// Binds a reactive array signal (loaded from the source, re-rendered by @each) PLUS
/// three auto-persisting mutators on ST._collections[$rows]: .insert / .delete / .reorder.
/// Each does optimistic-apply -> persist(op_id) over the named transport -> revert on
/// reject. Call them from an @on body: `@on click { $rows.insert(item); }`.
%macro data-collection-kind {
  %scope file | selector
  %order 248

  %form {
    @data collection $name:binding $type:typeref? from $src:expr via $transport:ident {
      initial: $initial:balanced(';')?
    }
  }

  %binds {
    data-source(
      name: $name,
      src: $src,
      default: $initial,
      refresh: 0
    ) -> {
      $data as $name,
      $loading as ${$name}_loading,
      $error as ${$name}_error,
      $reload as ${$name}_refetch
    }
    collection(
      name: $name,
      file: $src,
      transport: $transport,
      initial: $initial
    )
  }

  %registers binding($name, type: $type) {
    value: $name
    loading: ${$name}_loading
    error: ${$name}_error
    refetch: ${$name}_refetch
    source_kind: "collection"
    source_static: false
  }
}



/// ERROR SIBLING (BUG-264): a bare NUMBER in the type slot (`@data collection $x 5 from …`)
/// is a near-miss for the `$type:typeref?` optional. The productive
/// `data-collection-kind` form REJECTS it (strict-capture), so the value would
/// otherwise be silently dropped. This sibling reports it. Declared AFTER
/// `data-collection-kind`: the productive form wins cleanly on a valid type name.
%macro data-collection-invalid-type {
  %diagnostic
  %scope file | selector
  %order 248

  %form {
    @data collection $name:binding $type:number from $src:expr via $transport:ident {
      initial: $initial:balanced(';')?
    }
  }

  %binds {
    optional-clause-error(
      message: "a bare number is not a valid type",
      hint: "write a type name like `number`, `string`, or `Product[]` in the slot after `$name` — not a value"
    )
  }
}


/// @data source — a fetch whose URL is itself a SIGNAL (FEAT-076). Re-fetches
/// whenever the URL signal changes; republishes under $name. The keystone for
/// meta-renderers (e.g. the CMS admin's active collection = whichever file the
/// rail selected). static: false (resolves + re-resolves at runtime).
///
///   @data source $entries from $activeFile ;
%macro data-source-kind {
  %scope file | selector
  %order 250

  %form {
    @data source $name:binding $type:typeref? from $urlSignal:binding ;
  }

  %binds {
    reactive-source(name: $name, urlName: $urlSignal) -> {
      $value as $name
    }
  }

  %registers binding($name, type: $type) {
    value: $name
    source_kind: "reactive-source"
    source_static: false
  }
}




/// ERROR SIBLING (BUG-264): a bare NUMBER in the type slot (`@data source $x 5 from …`)
/// is a near-miss for the `$type:typeref?` optional. The productive
/// `data-source-kind` form REJECTS it (strict-capture), so the value would
/// otherwise be silently dropped. This sibling reports it. Declared AFTER
/// `data-source-kind`: the productive form wins cleanly on a valid type name.
%macro data-source-invalid-type {
  %diagnostic
  %scope file | selector
  %order 250

  %form {
    @data source $name:binding $type:number from $urlSignal:binding ;
  }

  %binds {
    optional-clause-error(
      message: "a bare number is not a valid type",
      hint: "write a type name like `number`, `string`, or `Product[]` in the slot after `$name` — not a value"
    )
  }
}


/// @data stream — server-sent events.
%macro data-stream-event-source {
  %scope file | selector
  %order 250
  %form { @data stream $name:binding $type:typeref? from $url:string ; }
  %binds {
    event-source(name: $name, url: $url) -> {
      $value as $name,
      $type as ${$name}_type,
      $state as ${$name}_state,
      $error as ${$name}_error
    }
  }
  %registers binding($name, type: $type) {
    value: $name
    source_kind: "event-source"
    source_static: false
  }
}


/// ERROR SIBLING (BUG-264): a bare NUMBER in the type slot (`@data stream $x 5 from "url" ;`)
/// is a near-miss for the `$type:typeref?` optional on the bodyless `@data stream`
/// (`data-stream-event-source`). The productive form REJECTS it (strict-capture),
/// so the value would otherwise be silently dropped. This sibling reports it.
/// Declared AFTER `data-stream-event-source`.
%macro data-stream-event-invalid-type {
  %diagnostic
  %scope file | selector
  %order 250
  %form {
    @data stream $name:binding $type:number from $url:string ;
  }
  %binds {
    optional-clause-error(
      message: "a bare number is not a valid type",
      hint: "write a type name like `number`, `string`, or `Product[]` in the slot after `$name` — not a value"
    )
  }
}


/// @data signal — kind 6: reactive OUTPUT, ONE correlated reply (≈ PureScript Aff).
/// PLAN-038 W1, pillar 2. The sibling of @data fetch on the OUTBOUND arc.
///
///   @data signal $add(text string) to $api {
///     send POST "/api/todos" { title: $text }
///     receive to AddResult {
///       200 => Created($.body as Todo);
///       422 => Invalid($.body as Errors);
///       _   => Failed($.statusText);
///     }
///     policy latest
///   }
///
/// Yields a CALLABLE $name + $name_pending / $name_error / $name_cancel, mirroring
/// @data fetch's $x / $x_loading / $x_error / $x_refetch. The signal binds the
/// `signal-call` primitive, which resolves its $host (the @host binding) to choose
/// the http/ws transport, sends the request, correlates the reply, and decodes it
/// through the `receive` arms into a response-sum variant. `policy` owns races
/// (latest = cancel in-flight). The optional `optimistic { }` group is reopened
/// per-scope by @handle.
///
/// params: the call signature (`$add(text string)`); $host: the @host binding
/// the signal sends to. send/receive are REQUIRED; policy/optimistic OPTIONAL.
%macro data-signal {
  %pipeline-consumed
  %scope file | selector
  %order 250

  %form {
    @data signal $name:binding($params:param_list) to $host:binding {
      ( $send:send_clause )
      ( $receive:receive_block )
      ( $policy:policy_clause )?
      ( optimistic { $optimistic:balanced('}') } )?
    }
  }

  %binds {
    signal-call(
      name: $name,
      params: $params,
      host: $host,
      verb: $verb,
      target: $target,
      body: $body,
      sum: $sum,
      arms: $arms,
      policy: $mode,
      timeout: $timeout,
      retry: $retry,
      ephemeral: $ephemeral
    ) -> {
      $call as $name,
      $pending as ${$name}_pending,
      $error as ${$name}_error,
      $cancel as ${$name}_cancel
    }
  }

  %registers binding($name) {
    value: $name
    pending: ${$name}_pending
    error: ${$name}_error
    cancel: ${$name}_cancel
    source_kind: "signal"
    source_static: false
    host: $host
  }
}


/// @data stream — kind 7: reactive OUTPUT, MANY replies over time (≈ Event).
/// PLAN-038 W1, pillar 2. Identical surface to @data signal; differs only in
/// cardinality — the `receive` arms fire repeatedly until a `final`-marked arm
/// ends the stream (a one-shot @data signal = all-arms-final implicitly).
%macro data-live-stream {
  %pipeline-consumed
  %scope file | selector
  %order 250

  // LiveView push_event is uncorrelated and transient: a stream has no `send`
  // clause. Its receive arms are the shared signal-call matcher vocabulary.
  %form {
    @data stream $name:binding $type:typeref? from $host:binding {
      ( $receive:receive_block )
    }
  }

  %binds {
    signal-call(
      name: $name,
      params: null,
      host: $host,
      verb: GET,
      target: "",
      body: null,
      sum: $sum,
      arms: $arms,
      policy: latest,
      timeout: null,
      retry: null,
      stream: true
    ) -> {
      $call as $name,
      $pending as ${$name}_pending,
      $error as ${$name}_error,
      $cancel as ${$name}_cancel
    }
  }

  %registers binding($name) {
    value: $name
    pending: ${$name}_pending
    error: ${$name}_error
    cancel: ${$name}_cancel
    source_kind: "live-stream"
    source_static: false
    host: $host
  }
}


/// ERROR SIBLING (BUG-264): a bare NUMBER in the type slot (`@data stream $x 5 from $h { … }`)
/// is a near-miss for the `$type:typeref?` optional on the BODY `@data stream`
/// (`data-live-stream`). The productive form REJECTS it (strict-capture), so the
/// value would otherwise be silently dropped. This sibling reports it. Declared
/// AFTER `data-live-stream`: the productive form wins cleanly on a valid type name.
%macro data-live-stream-invalid-type {
  %diagnostic
  %scope file | selector
  %order 250

  %form {
    @data stream $name:binding $type:number from $host:binding {
      ( $receive:receive_block )
    }
  }

  %binds {
    optional-clause-error(
      message: "a bare number is not a valid type",
      hint: "write a type name like `number`, `string`, or `Product[]` in the slot after `$name` — not a value"
    )
  }
}


/// @data stream with a send clause keeps the established multi-reply transport
/// surface. The passive `from $host` form above is reserved for LiveView pushes.
%macro data-stream {
  %pipeline-consumed
  %scope file | selector
  %order 250

  %form {
    @data stream $name:binding($params:param_list) to $host:binding {
      ( $send:send_clause )
      ( $receive:receive_block )
      ( $policy:policy_clause )?
      ( optimistic { $optimistic:balanced('}') } )?
    }
  }

  %binds {
    signal-call(
      name: $name, params: $params, host: $host, verb: $verb, target: $target,
      body: $body, sum: $sum, arms: $arms, policy: $mode, timeout: $timeout,
      retry: $retry, ephemeral: $ephemeral, stream: true
    ) -> {
      $call as $name, $pending as ${$name}_pending, $error as ${$name}_error,
      $cancel as ${$name}_cancel
    }
  }

  %registers binding($name) {
    value: $name
    pending: ${$name}_pending
    error: ${$name}_error
    cancel: ${$name}_cancel
    source_kind: "stream"
    source_static: false
    host: $host
  }
}

/// The host module + assign + target triple is registered by `signal-subscribe`;
/// assign diffs enter through the existing `SpacetimeLocal` channel, so no second
/// decode/event vocabulary is introduced.
///
/// Like every sibling @data kind, subscribe accepts an OPTIONAL `$type:typeref?`
/// (FUP-144) so the subscribed signal is typed — the binding registers with its
/// shape and a named `@type` used only here is no longer "defined but never used".
/// A companion body form (`data-subscribe-seeded`) adds an explicit pre-first-diff
/// seed; the first server assign diff overwrites it, so the seed is never
/// authoritative server data (it just makes reads total before the first diff).
///
///   @data subscribe $count from $counter : count ;              // untyped → seed null
///   @data subscribe $count number from $counter : count ;       // typed → seed 0
///   @data subscribe $form Env from $counter : form ;            // typed → seed Env's
///                                                               //   ZERO VALUE (nested)
///   @data subscribe $count number from $counter : count {       // explicit seed WINS
///     initial: 0;
///   }
///
/// FUP-149 (PLAN-119 W3): a TYPED bodyless subscribe is AUTO-SEEDED with its
/// type's zero value — number→0, string→"", record→{fields:zero}, etc. — so its
/// (possibly NESTED) reads are total from mount without a hand-written seed. The
/// zero value is built structurally by the compiler (TypeRegistry::zero_value)
/// and serialized to a JS literal, so a nested envelope works even though a
/// hand-written nested `{ initial: {…} }` literal is still blocked by the
/// balanced-capture limitation (BUG-228 / PLAN-120). An explicit `{ initial: … }`
/// body (the `data-subscribe-seeded` macro) OVERRIDES the derived seed.
%macro data-subscribe {
  %scope file | selector
  %order 250

  %form {
    @data subscribe $name:binding $type:typeref? from $host:binding : $assign:ident ;
  }

  %binds {
    // The target is a normal local signal; the subscription only supplies its
    // server-driven updates through that established channel. The seed is the
    // PRE-FIRST-DIFF value: a compiler pass (FUP-149 / PLAN-119 W3) fills $seed
    // with the TYPE'S ZERO VALUE for a typed subscribe (so nested reads are
    // total from mount, not guarded against null) and with `null` for an
    // untyped one (byte-identical to the pre-FUP-149 hardcoded `initial: null`).
    // signal-subscribe overwrites it via SpacetimeLocal on the first diff, so
    // the seed is never authoritative server data.
    local-state-impl(name: $name, initial: $seed) -> { $state as $name }
    signal-subscribe(name: $name, host: $host, assign: $assign)
  }

  %registers binding($name, type: $type) {
    value: $name
    source_kind: "live-subscribe"
    source_static: false
    host: $host
    assign: $assign
  }
}


/// @data subscribe (with seed body) — a typed subscription plus an explicit
/// PRE-FIRST-DIFF seed. Companion to the bodyless `@data subscribe` (mirrors
/// @data fetch vs fetch-opts: a distinct %macro per body shape, since a %macro
/// carries one %form). The seed is the value the signal holds BEFORE the first
/// server assign diff arrives, so a typed object subscribe (e.g. a form
/// envelope) has a well-shaped value and its nested reads (`$form.errors.email`)
/// are TOTAL from mount rather than guarded against null. signal-subscribe
/// overwrites it via SpacetimeLocal on the first diff — the seed is never
/// authoritative server data. FUP-144.
///
///   @data subscribe $form FormEnvelope from $auth : form {
///     initial: { "values": {}, "errors": {}, "valid": false };
///   }
%macro data-subscribe-seeded {
  %scope file | selector
  // Match bodyless subscribe's %order (250): signal-subscribe must EMIT AFTER the
  // @host registration (also 250) so __stHosts[host].module is populated when the
  // subscription IIFE runs — otherwise it keys under the empty-string module and
  // the bridge never routes diffs to it. The two subscribe %forms disambiguate by
  // SHAPE (body vs bodyless), not %order, so sharing 250 is safe.
  %order 250

  %form {
    @data subscribe $name:binding $type:typeref? from $host:binding : $assign:ident {
      initial: $seed:balanced(';')?
    }
  }

  %binds {
    local-state-impl(name: $name, initial: $seed) -> { $state as $name }
    signal-subscribe(name: $name, host: $host, assign: $assign)
  }

  %registers binding($name, type: $type) {
    value: $name
    source_kind: "live-subscribe"
    source_static: false
    host: $host
    assign: $assign
  }
}
