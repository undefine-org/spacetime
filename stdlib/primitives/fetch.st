/// Fetch Primitive
///
/// Wraps the Fetch API for reactive data loading.
/// Supports caching, refetching, and error handling.

%primitive fetch(
  /// URL to fetch
  url: string,
  /// Fetch options (method, headers, body, etc.)
  options: object = {},
  /// Cache duration (0 = no cache)
  cache: number = 0,
  /// Fetch immediately on creation
  autoFetch: bool = true,
  /// Transform function for response
  transform: ("json" | "text" | "blob" | "arrayBuffer") = "json"
) {
  %emit js {
    let cachedData = null;
    let cacheTime = 0;
    let abortController = null;

    const doFetch = async () => {
      const now = Date.now();

      // Check cache validity
      if (cachedData !== null && %cache > 0 && (now - cacheTime) < %cache) {
        %yield cachedData -> $data;
        %yield false -> $loading;
        return cachedData;
      }

      // Abort any in-flight request
      if (abortController) {
        abortController.abort();
      }
      abortController = new AbortController();

      %yield true -> $loading;
      %yield null -> $error;

      try {
        const response = await fetch('%url', {
          ...%options,
          signal: abortController.signal
        });

        %yield response.status -> $status;
        %yield response.ok -> $ok;
        %yield Object.fromEntries(response.headers.entries()) -> $headers;

        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(`HTTP ${response.status}: ${errorText}`);
        }

        let data;
        switch (%transform) {
          case 'text':
            data = await response.text();
            break;
          case 'blob':
            data = await response.blob();
            break;
          case 'arrayBuffer':
            data = await response.arrayBuffer();
            break;
          default:
            data = await response.json();
        }

        cachedData = data;
        cacheTime = now;

        %yield data -> $data;
        %yield false -> $loading;

        return data;

      } catch (e) {
        if (e.name === 'AbortError') {
          // Request was aborted, don't treat as error
          return null;
        }

        %yield e -> $error;
        %yield false -> $loading;
        %yield null -> $data;

        throw e;
      }
    };

    // Invalidate cache
    const invalidate = () => {
      cachedData = null;
      cacheTime = 0;
    };

    // Expose control methods
    %yield doFetch -> $refetch;
    %yield invalidate -> $invalidate;

    // Initial fetch
    if (%autoFetch) {
      doFetch();
    } else {
      %yield false -> $loading;
      %yield null -> $data;
      %yield null -> $error;
    }

    %cleanup {
      if (abortController) {
        abortController.abort();
      }
    }
  }

  %exports {
    /// The fetched data
    $data: any           // fetched data
    /// Whether a fetch is in progress
    $loading: bool       // fetch in progress
    /// Error if fetch failed
    $error: any?         // error if failed
    /// HTTP status code
    $status: number      // HTTP status code
    $ok: bool            // response.ok
    $headers: object     // response headers
    /// Function to trigger a refetch
    $refetch: fn() ~> effect         // trigger refetch
    $invalidate: fn() ~> effect      // clear cache
  }
}


/// GraphQL Fetch Primitive
///
/// Specialized fetch for GraphQL queries.

%primitive graphql(
  /// GraphQL endpoint URL
  url: string,
  /// GraphQL query string
  query: string,
  /// Query variables
  variables: object = {},
  /// Additional headers
  headers: object = {}
) {
  %emit js {
    let abortController = null;

    const execute = async (overrideVariables = null) => {
      if (abortController) {
        abortController.abort();
      }
      abortController = new AbortController();

      %yield true -> $loading;
      %yield null -> $error;

      try {
        const response = await fetch('%url', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            ...%headers
          },
          body: JSON.stringify({
            query: %query,
            variables: overrideVariables || %variables
          }),
          signal: abortController.signal
        });

        if (!response.ok) {
          throw new Error(`HTTP ${response.status}`);
        }

        const result = await response.json();

        if (result.errors && result.errors.length > 0) {
          const error = new Error(result.errors.map(e => e.message).join(', '));
          error.graphQLErrors = result.errors;
          throw error;
        }

        %yield result.data -> $data;
        %yield false -> $loading;

        return result.data;

      } catch (e) {
        if (e.name === 'AbortError') return null;

        %yield e -> $error;
        %yield false -> $loading;
        %yield null -> $data;

        throw e;
      }
    };

    %yield execute -> $execute;
    %yield execute -> $refetch;

    // Auto-execute
    execute();

    %cleanup {
      if (abortController) {
        abortController.abort();
      }
    }
  }

  %exports {
    $data: any           // query result data
    $loading: bool       // query in progress
    $error: any?         // error with graphQLErrors property
    $execute: fn() ~> effect         // execute with optional variable override
    $refetch: fn() ~> effect         // re-execute query
  }
}


/// Async Action Primitive
///
/// Wraps an async action with reactive signals for pending/success/error states.
/// The action is not executed immediately - call $run() to execute.

%primitive async-action(
  /// The async action to execute (Promise or async function)
  action: expr
) {
  %emit js {
    %yield false -> $pending;
    %yield false -> $ok;
    %yield null -> $error;

    const run = async () => {
      %yield true -> $pending;
      %yield false -> $ok;
      %yield null -> $error;

      try {
        await %action;
        %yield false -> $pending;
        %yield true -> $ok;
      } catch (e) {
        %yield false -> $pending;
        %yield false -> $ok;
        %yield e -> $error;
      }
    };

    %yield run -> $run;
  }

  %exports {
    /// Function to execute the action
    $run: fn() ~> effect          // execute the action
    /// True while action is executing
    $pending: bool    // action in progress
    /// True if action succeeded (resets on next run)
    $ok: bool         // action succeeded
    /// Error object if action failed
    $error: any?      // error if failed
  }
}
