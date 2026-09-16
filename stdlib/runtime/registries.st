/// Runtime Registry Definitions
///
/// These declare the runtime namespaces and how they work per target.
/// Macros reference these by name via %resolves.
///
/// Example usage in a macro:
///   %resolves {
///     $name -> functions
///   }

%runtime-registry functions {
  %target js {
    %namespace { ST.functions }
    %init { ST.functions = ST.functions || {}; }
    %register($name, $value) { ST.functions[$name] = $value; }
    %call($name, $args) { ST.functions[$name]($args) }
    %access($name) { ST.functions[$name] }
  }
}

%runtime-registry filters {
  %target js {
    %namespace { ST.filters }
    %init { ST.filters = ST.filters || {}; }
    %register($name, $fn) { ST.filters[$name] = $fn; }
    %access($name) { ST.filters[$name] }
  }
}

/// Editable mark registry (PLAN-031 / FEAT-097).
///
/// Inline formatting spans available to an `@editable` surface (bold, italic,
/// link, custom marks). Each entry is a schema descriptor `{ tag, attrs, sel }`
/// derived from the mark's projection body, plus the template factory name (a
/// mark IS a template). The editor reads this to know which marks exist, what
/// HTML they project to, and which typed attrs they carry (paste-lift + toolbar).
%runtime-registry editable-marks {
  %target js {
    %namespace { ST.editable.marks }
    %init { ST.editable = ST.editable || {}; ST.editable.marks = ST.editable.marks || {}; }
    %register($name, $schema) { ST.editable.marks[$name] = $schema; }
    %access($name) { ST.editable.marks[$name] }
  }
}

/// Editable block registry (PLAN-031 / FEAT-097).
///
/// Block-level containers available to an `@editable` surface (paragraph,
/// heading, figure, custom blocks). Same descriptor shape as marks; the editor
/// distinguishes block vs inline by which registry an entry lives in.
%runtime-registry editable-blocks {
  %target js {
    %namespace { ST.editable.blocks }
    %init { ST.editable = ST.editable || {}; ST.editable.blocks = ST.editable.blocks || {}; }
    %register($name, $schema) { ST.editable.blocks[$name] = $schema; }
    %access($name) { ST.editable.blocks[$name] }
  }
}
