/// Fetch Primitive
///
/// Wraps the Fetch API for reactive data loading.
/// Supports caching, refetching, and error handling.
///
/// @param url - URL to fetch
/// @param options - Fetch options (method, headers, body, etc.)
/// @param cache - Cache duration (0 = no cache)
/// @param autoFetch - Fetch immediately on creation, default true
/// @param transform - Transform function for response (default: json)
///
/// @yields $data - The fetched data
/// @yields $loading - Whether a fetch is in progress
/// @yields $error - Error if fetch failed
/// @yields $status - HTTP status code
/// @yields $refetch - Function to trigger a refetch

%primitive fetch(
  url: string,
  options: object = {},
  cache: number = 0,
  autoFetch: bool = true,
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
    $data: any           // fetched data
    $loading: bool       // fetch in progress
    $error: any?         // error if failed
    $status: number      // HTTP status code
    $ok: bool            // response.ok
    $headers: object     // response headers
    $refetch: fn() ~> effect         // trigger refetch
    $invalidate: fn() ~> effect      // clear cache
  }
}


/// GraphQL Fetch Primitive
///
/// Specialized fetch for GraphQL queries.
///
/// @param url - GraphQL endpoint URL
/// @param query - GraphQL query string
/// @param variables - Query variables
/// @param headers - Additional headers

%primitive graphql(
  url: string,
  query: string,
  variables: object = {},
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
///
/// @param action - The async action to execute (Promise or async function)
///
/// @yields $run - Function to execute the action
/// @yields $pending - True while action is executing
/// @yields $ok - True if action succeeded (resets on next run)
/// @yields $error - Error object if action failed

%primitive async-action(action: expr) {
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
    $run: fn() ~> effect          // execute the action
    $pending: bool    // action in progress
    $ok: bool         // action succeeded
    $error: any?      // error if failed
  }
}
