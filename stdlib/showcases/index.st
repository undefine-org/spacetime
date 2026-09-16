/// Spacetime showcase tree — curated patterns indexed for the design skill.
///
/// Each `<scenario>/<style>.st` file under this tree is a self-contained pattern
/// whose `///` header (scenario / style / school / description / dependencies /
/// screenshot / useWhen) is read AS DATA by `@data declarations … from
/// template` — a page enumerates the patterns and their metadata through the
/// ordinary language surface, with no CLI and no bespoke registry.
///
/// This file is the tree's MANIFEST: importing it makes every pattern (and its
/// metadata) visible to the importing page. `demos/showcase-gallery/` is the
/// rendered catalog and generates its specimen list from exactly that.
///
/// See SCHEMA.md for the doc-comment contract and CATALOG.md for the
/// human-readable index.

@import "./_schools.st"
@import "./hero/minimal.st"
@import "./hero/editorial.st"
@import "./hero/maximal.st"
@import "./hero/architecture.st"
@import "./hero/motion.st"
@import "./hero/eastern.st"
@import "./nav/minimal.st"
@import "./cta/minimal.st"
@import "./footer/minimal.st"
@import "./features-grid/minimal.st"
@import "./cta/magnetic.st"
@import "./features-grid/tabbed.st"
@import "./pricing/interactive.st"
@import "./nav/marquee.st"
@import "./cta/editorial.st"
