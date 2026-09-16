/// The PROJECT OVERLAY: declarations this project owns.
///
/// Auto-loaded into the MetaRegistry before any page here compiles, and never
/// served as a page. A type declared here joins the pill's picker, MCP
/// validation, and `check`'s roster with no Rust change — that is the whole
/// point of types-as-data.

/// A project-specific type, to show that the roster is genuinely extensible.
/// `reviewer` is REQUIRED: a review nobody is named for is a review that does
/// not happen.
%comment_type brand-review {
  %label "Brand review"
  %docs "Copy or visual that needs sign-off from brand before it ships."
  %field reviewer string
  %field due string?
  %color "#b06ee8"
  %agent_hint "Check the text against the brand guidelines, then reply with
               exactly what you changed. If the call needs a human decision,
               leave it open and say what the options are."
}
