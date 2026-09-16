/// Spacetime `editable` module — structured rich-text input (PLAN-031).
///
/// Provides `@editable`, a structured rich-text surface whose document is a
/// constrained AST (not contenteditable innerHTML), and the `@editable-mark` /
/// `@editable-block` macros that define the inline + block vocabulary that surface
/// accepts. A mark IS a template whose first param is the selection and whose body
/// is the HTML projection — render it forward, match it backward (paste-lift),
/// type its free-param holes.
///
/// No hand-rolled rich-text engine in Rust and no privileged built-in mark set:
/// every mark/block is an ordinary registered construct, and the editable schema +
/// runtime registries fall out of what is registered (AGENTS self-describing rule).
///
/// @example
/// @import "stdlib/editable"
///
/// @editable-mark &highlight(&sel) { <mark>`&sel`</mark> }   // custom mark
///
/// .body {
///   @editable(bind: $post.body) {
///     marks: &bold &italic &link &highlight;
///     blocks: &p &h2 &figure;
///   }
/// }
///
/// See PLAN-031 + @research/spacetime-editable-spec.org.

@import "./index.st"
