/// I/O Primitives for Spacetime Scripts
///
/// Provides file system operations for scripts.
/// Note: These require the headless runtime with Node.js-like fs access.


// =============================================================================
// Types
// =============================================================================

@type FileInfo {
  path: string;
  exists: boolean;
  isFile: boolean;
  isDirectory: boolean;
  size: number;
}


// =============================================================================
// Output Primitive
// =============================================================================

/// Collects output for writing to files or stdout
///
/// @param &container - Script container
/// @yields $write - Write content to a path (or stdout if path is "-")
/// @yields $writeJson - Write JSON content to a path
/// @yields $getOutputs - Get all pending outputs

%primitive output_collector(&container) {
  %emit js {
    globalThis.__st_outputs = globalThis.__st_outputs || [];

    const write = (path, content) => {
      globalThis.__st_outputs.push({ type: 'text', path, content });
    };

    const writeJson = (path, data) => {
      globalThis.__st_outputs.push({
        type: 'json',
        path,
        content: JSON.stringify(data, null, 2)
      });
    };

    const getOutputs = () => [...globalThis.__st_outputs];

    globalThis.__st_write = write;
    globalThis.__st_write_json = writeJson;
    globalThis.__st_get_outputs = getOutputs;

    %yield write -> $write;
    %yield writeJson -> $writeJson;
    %yield getOutputs -> $getOutputs;
  }

  %exports {
    $write: fn(path: string, content: string) ~> effect
    $writeJson: fn(path: string, data: object) ~> effect
    $getOutputs: fn() ~> array
  }
}


// =============================================================================
// @output Macro
// =============================================================================

/// Write transformed content to a file
///
/// @param $path - Output file path (use "-" for stdout)
/// @param $content - Content to write
///
/// @example
/// @output "./migrated.st" content: $transformed

%macro output {

  %form {
    @output $path:string content: $content:expr
  }

  %emit js {
    globalThis.__st_write(%$path, %$content);
  }
}


// =============================================================================
// @output-json Macro
// =============================================================================

/// Write JSON data to a file
///
/// @param $path - Output file path
/// @param $data - Object to serialize as JSON
///
/// @example
/// @output-json "./analysis.json" data: { edits: $edits, stats: $stats }

%macro output-json {

  %form {
    @output-json $path:string data: $data:expr
  }

  %emit js {
    globalThis.__st_write_json(%$path, %$data);
  }
}
