// GH-21: a `.st`-only entry sets the page head (title, html lang, meta) and
// registers remote fonts — with NO authored index.html.
//
// The page shell owns the real `<html>`/`<head>`. This entry authors a top-level
// `<html lang="fr">` wrapper + head-eligible tags; render_page_shell extracts
// them and feeds the generated shell (host spelling must work — see
// docs/language/css-interop.md). The `lang` reaches `<html lang>`, the title /
// meta / font-preconnect link are hoisted into `<head>`, and the wrapper is
// stripped so only the body content ships in <body>.
<html lang="fr">
  <title>Accueil — Backdesk</title>
  <meta name="description" content="Réservation d'hôtel simple et directe">
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <main class="page">
    <h1>Bienvenue chez Backdesk</h1>
    <p>Un séjour sans friction.</p>
  </main>
</html>
