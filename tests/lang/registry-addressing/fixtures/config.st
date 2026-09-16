/// Fixture module for the registry-addressing matrices (PLAN-117).
///
/// A "state module": a file whose contribution to a page is CELLS, not markup.
/// Three cells with deliberately different profiles, so the provenance and
/// fold/eliminate matrices have something real to assert against:
///
///   $host          write-free literal  -> const candidate (W4 folds it)
///   $session       written by importers -> stays a live reactive cell
///   $retryBudget   never read anywhere  -> dead candidate (W4 eliminates it)

$host string: "https://api.example.com";
$session string: "anonymous";
$retryBudget number: 3;
