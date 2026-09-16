/// Scripting Framework for Spacetime
///
/// Provides primitives and macros for writing source transformation scripts.
/// Scripts can parse, analyze, and transform .st files with full AST access.
///
/// @example
/// @script "Rename directive" {
///   @find "@old-name" {
///     @replace with: "@new-name"
///   }
/// }


// =============================================================================
// Types
// =============================================================================

@type FileContext {
  path: string;
  source: string;
  ast: object;
}

@type SourceEdit {
  start: number;
  end: number;
  replacement: string;
}

@type ScriptResult {
  file: string;
  edits: SourceEdit[];
  transformed: string;
}


// =============================================================================
// File Context Primitive
// =============================================================================

/// Provides access to the current file being processed
///
/// @param &container - Script container element
/// @yields $source - Raw source code string
/// @yields $ast - Parsed AST as JSON object
/// @yields $path - File path string
/// @yields $forms - FormRegistry data for directive signatures

%primitive file_context(&container) {
  %emit js {
    // These globals are injected by the script runner before execution:
    //   __st_file_source  - raw source string
    //   __st_file_ast     - parsed AST as JSON
    //   __st_file_path    - file path
    //   __st_file_forms   - form registry data

    %yield globalThis.__st_file_source -> $source;
    %yield globalThis.__st_file_ast -> $ast;
    %yield globalThis.__st_file_path -> $path;
    %yield globalThis.__st_file_forms || {} -> $forms;
  }

  %exports {
    $source: string
    $ast: object
    $path: string
    $forms: object
  }
}


// =============================================================================
// Script Runner Primitive
// =============================================================================

/// Manages script registration and execution
///
/// @param &container - Script container element
/// @yields $registerScript - Function to register a named script
/// @yields $runScripts - Function to execute all registered scripts

%primitive script_runner(&container) {
  %emit js {
    globalThis.__st_scripts = globalThis.__st_scripts || [];
    globalThis.__st_edits = globalThis.__st_edits || [];

    const registerScript = (name, fn) => {
      globalThis.__st_scripts.push({ name, fn });
    };

    const runScripts = async () => {
      const results = [];
      for (const script of globalThis.__st_scripts) {
        try {
          const result = await script.fn();
          results.push({ name: script.name, success: true, result });
          console.log(`PASS: ${script.name}`);
        } catch (e) {
          results.push({ name: script.name, success: false, error: e.message });
          console.error(`FAIL: ${script.name}: ${e.message}`);
        }
      }
      return results;
    };

    globalThis.__spacetime_register_script = registerScript;
    globalThis.__spacetime_run_scripts = runScripts;

    %yield registerScript -> $registerScript;
    %yield runScripts -> $runScripts;
  }

  %exports {
    $registerScript: fn(name: string, callback: any) ~> effect
    $runScripts: fn() ~> array
  }
}


// =============================================================================
// Edit Collector Primitive
// =============================================================================

/// Collects source edits for batch application
///
/// @param &container - Script container
/// @yields $addEdit - Add a source edit (start, end, replacement)
/// @yields $getEdits - Get all collected edits
/// @yields $applyEdits - Apply edits to source and return transformed

%primitive edit_collector(&container) {
  %emit js {
    const edits = [];

    const addEdit = (start, end, replacement) => {
      edits.push({ start, end, replacement });
    };

    const getEdits = () => [...edits];

    const applyEdits = (source) => {
      // Sort edits in reverse order (end to start) to preserve positions
      const sorted = [...edits].sort((a, b) => b.start - a.start);
      let result = source;
      for (const edit of sorted) {
        result = result.slice(0, edit.start) + edit.replacement + result.slice(edit.end);
      }
      return result;
    };

    globalThis.__st_add_edit = addEdit;
    globalThis.__st_get_edits = getEdits;
    globalThis.__st_apply_edits = applyEdits;

    %yield addEdit -> $addEdit;
    %yield getEdits -> $getEdits;
    %yield applyEdits -> $applyEdits;
  }

  %exports {
    $addEdit: fn(start: number, end: number, replacement: string) ~> effect
    $getEdits: fn() ~> array
    $applyEdits: fn(source: string) ~> string
  }
}


// =============================================================================
// @script Macro
// =============================================================================

/// Define a named transformation script
///
/// @param $name - Script name for reporting
/// @body - Script body with access to $source, $ast, $path, $forms
///
/// @example
/// @script "My transformation" {
///   // $source, $ast, $path available here
///   console.log("Processing:", $path);
/// }

%macro script {

  %form {
    @script $name:string {
      $body:template
    }
  }

  %binds {
    file_context(&self) -> { $source, $ast, $path, $forms }
    edit_collector(&self) -> { $addEdit, $getEdits, $applyEdits }
  }

  %emit js {
    globalThis.__spacetime_register_script(%$name, async () => {
      // File context is available via bindings
      const source = globalThis.__st_file_source;
      const ast = globalThis.__st_file_ast;
      const path = globalThis.__st_file_path;
      const forms = globalThis.__st_file_forms || {};

      // Edit functions available
      const addEdit = globalThis.__st_add_edit;
      const getEdits = globalThis.__st_get_edits;
      const applyEdits = globalThis.__st_apply_edits;

      // Execute script body
      %$body

      // Return result
      return {
        path,
        edits: getEdits(),
        transformed: applyEdits(source)
      };
    });
  }
}


// =============================================================================
// @find Macro - Query AST nodes
// =============================================================================

/// Find and iterate over AST nodes matching a pattern
///
/// @param $pattern - Pattern to match (directive name, node type, etc.)
/// @body - Handler executed for each match, with $node and $span available
///
/// @example
/// @find "@fade-in" {
///   console.log("Found fade-in at", $span);
/// }

%macro find {

  %form {
    @find $pattern:string {
      $body:template
    }
  }

  %emit js {
    const pattern = %$pattern;
    const ast = globalThis.__st_file_ast;

    // Simple pattern matching - looks for macro calls by name
    const findNodes = (node, results = []) => {
      if (!node || typeof node !== 'object') return results;

      // Check if this is a macro call matching our pattern
      if (node.type === 'macro_call' || node.name) {
        const name = node.name || '';
        if (pattern.startsWith('@') && name === pattern.slice(1)) {
          results.push({ node, span: node.span || {} });
        } else if (name.includes(pattern)) {
          results.push({ node, span: node.span || {} });
        }
      }

      // Recurse into children
      for (const key of Object.keys(node)) {
        const val = node[key];
        if (Array.isArray(val)) {
          val.forEach(item => findNodes(item, results));
        } else if (typeof val === 'object' && val !== null) {
          findNodes(val, results);
        }
      }

      return results;
    };

    const matches = findNodes(ast);
    for (const { node: $node, span: $span } of matches) {
      %$body
    }
  }
}


// =============================================================================
// @replace Macro - Text replacement
// =============================================================================

/// Replace text patterns in source
///
/// @param $pattern - Text or regex pattern to find
/// @param $replacement - Replacement text
///
/// @example
/// @replace("@old-directive", "@new-directive")

%macro replace {

  %form {
    @replace($pattern:string, $replacement:string)
  }

  %emit js {
    const pattern = %$pattern;
    const replacement = %$replacement;
    const source = globalThis.__st_file_source;

    // Find all occurrences and add edits
    let index = 0;
    while ((index = source.indexOf(pattern, index)) !== -1) {
      globalThis.__st_add_edit(index, index + pattern.length, replacement);
      index += pattern.length;
    }
  }
}


// =============================================================================
// @replace-regex Macro - Regex replacement
// =============================================================================

/// Replace patterns using regular expressions
///
/// @param $pattern - Regex pattern string
/// @param $replacement - Replacement (supports $1, $2 capture groups)
/// @param $flags - Optional regex flags (default: "g")
///
/// @example
/// @replace-regex("duration:\\s*(\\d+)ms", "duration: ${1}ms /* migrated */")

%macro replace-regex {

  %form {
    @replace-regex($pattern:string, $replacement:string)
  }

  %emit js {
    const pattern = %$pattern;
    const replacement = %$replacement;
    const flags = "g";  // Default to global
    const source = globalThis.__st_file_source;

    const regex = new RegExp(pattern, flags);
    let match;
    const edits = [];

    // Reset regex state
    regex.lastIndex = 0;

    while ((match = regex.exec(source)) !== null) {
      // Build replacement with capture groups
      let replaced = replacement;
      for (let i = 0; i <= match.length; i++) {
        replaced = replaced.replace(new RegExp('\\$' + i, 'g'), match[i] || '');
      }

      globalThis.__st_add_edit(match.index, match.index + match[0].length, replaced);

      // Prevent infinite loop for zero-length matches
      if (match[0].length === 0) regex.lastIndex++;
    }
  }
}


// =============================================================================
// @log Macro - Debug logging
// =============================================================================

/// Log a message during script execution
///
/// @param $message - Message to log

%macro log {

  %form {
    @log $message:string
  }

  %emit js {
    console.log('[script]', %$message);
  }
}


// =============================================================================
// @assert Macro - Script assertions
// =============================================================================

/// Assert a condition, throwing if false
///
/// @param $condition - Condition to check
/// @param $message - Error message if assertion fails

%macro assert {

  %form {
    @assert $condition:expr message: $message:string = "Assertion failed"
  }

  %emit js {
    if (!(%$condition)) {
      throw new Error(%$message);
    }
  }
}
