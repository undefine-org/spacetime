/// Pipeline-only sink for a DATA-defined error sibling.
///
/// The pipeline consumes this before primitive resolution and turns its
/// `message`/`hint` arguments into E0955. It deliberately has no emitted body:
/// a matched malformed clause must produce a compile error, never JavaScript.
%primitive optional-clause-error(
  rule: string = "always",
  value: string = "",
  message: string,
  hint: string
) {
}
