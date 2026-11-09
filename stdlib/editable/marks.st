/// The inline marks every page can use, as ordinary forms.
///
/// A mark is not a construct of its own: it is a `markup` form whose signature
/// wraps text — one content slot, typed scalar params, one root element of
/// phrasing content. An editor offers every such form in scope (these, then
/// the page's own, which win by name) and writes the form's projection around
/// a selection; in prose, the prose's own syntax for the same element.
///
/// How each is presented is inferred (label from the name, doc from `///`,
/// each param's control from the scalar table) and overridden with `@cms`,
/// the display-hint macro the admin already uses for types.
///
/// A page adds its own the same way:
///   @form markup --accent(&sel) { <mark class="accent">`&sel`</mark> }
///   @cms(--accent) { icon: "✦"; shortcut: "mod+e"; }

/// Words that carry the sentence.
@form markup --bold(&sel) { <strong>`&sel`</strong> }
@cms(--bold) { icon: "B"; shortcut: "mod+b"; }

/// Stress, a title, a word in another voice.
@form markup --italic(&sel) { <em>`&sel`</em> }
@cms(--italic) { icon: "I"; shortcut: "mod+i"; }

/// Words that no longer hold, left visible.
@form markup --strike(&sel) { <s>`&sel`</s> }
@cms(--strike) { icon: "S"; shortcut: "mod+shift+x"; }

/// A name from code: a function, a file, a key.
@form markup --code(&sel) { <code>`&sel`</code> }
@cms(--code) { icon: "</>"; shortcut: "mod+e"; }

/// A link. Its `url` is a link, never a script.
@form markup --link(&sel, $href url) { <a href="`$href`">`&sel`</a> }
@cms(--link) { icon: "↗"; shortcut: "mod+k"; }
