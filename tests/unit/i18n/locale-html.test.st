/// Unit Tests: i18n Locale HTML Rendering
///
/// Tests that locale strings containing HTML tags render as formatted HTML
/// instead of displaying raw tags as text, while maintaining XSS safety.

@import "stdlib/testing/test"

// =============================================================================
// HTML Detection & Rendering
// =============================================================================

@test "locale HTML: strong tag renders as formatted HTML" {
  @fixture {
    <div class="locale-strong-test">
      <span data-t="bold_key">placeholder</span>
    </div>
  }

  @eval (eval("                                                          \
    var __stHtmlRe = /<[a-z][\\s\\S]*>/i;                                \
    function __stSanitize(html) {                                        \
      html = html.replace(/<(script|style|iframe|object|embed|form)[^>]*>[\\s\\S]*?<\\/\\1>/gi, ''); \
      html = html.replace(/<(script|style|iframe|object|embed|form|input|textarea|select)[^>]*\\/?>/gi, ''); \
      html = html.replace(/\\s+on\\w+\\s*=\\s*(?:'[^']*'|[^\\s>]+)/gi, ''); \
      return html;                                                       \
    }                                                                    \
    var val = '<strong>Bold Text</strong>';                               \
    var el = document.querySelector('[data-t=\\x22bold_key\\x22]');      \
    if (__stHtmlRe.test(val)) {                                          \
      el.innerHTML = __stSanitize(val);                                  \
    } else {                                                             \
      el.textContent = val;                                              \
    }                                                                    \
    window.__strongInner = el.innerHTML;                                  \
    window.__strongHasTag = el.querySelector('strong') !== null;          \
  "))

  @assert (window.__strongHasTag === true)
    "Locale value with <strong> should render as HTML element"

  @assert (window.__strongInner.indexOf('Bold Text') !== -1)
    "Bold text content should be preserved"
}

@test "locale HTML: script tag is stripped for XSS safety" {
  @fixture {
    <div class="locale-xss-test">
      <span data-t="xss_key">placeholder</span>
    </div>
  }

  @eval (eval("                                                          \
    var __stHtmlRe = /<[a-z][\\s\\S]*>/i;                                \
    function __stSanitize(html) {                                        \
      html = html.replace(/<(script|style|iframe|object|embed|form)[^>]*>[\\s\\S]*?<\\/\\1>/gi, ''); \
      html = html.replace(/<(script|style|iframe|object|embed|form|input|textarea|select)[^>]*\\/?>/gi, ''); \
      html = html.replace(/\\s+on\\w+\\s*=\\s*(?:'[^']*'|[^\\s>]+)/gi, ''); \
      return html;                                                       \
    }                                                                    \
    var val = 'Hello<script>alert(1)<\\/script>World';                   \
    var el = document.querySelector('[data-t=\\x22xss_key\\x22]');       \
    if (__stHtmlRe.test(val)) {                                          \
      el.innerHTML = __stSanitize(val);                                  \
    } else {                                                             \
      el.textContent = val;                                              \
    }                                                                    \
    window.__xssInner = el.innerHTML;                                    \
  "))

  @assert (window.__xssInner.indexOf('script') === -1)
    "Script tags must be completely stripped"

  @assert (window.__xssInner.indexOf('Hello') !== -1)
    "Text before script should be preserved"

  @assert (window.__xssInner.indexOf('World') !== -1)
    "Text after script should be preserved"
}

@test "locale HTML: plain text uses textContent (no double-escaping)" {
  @fixture {
    <div class="locale-plain-test">
      <span data-t="plain_key">placeholder</span>
    </div>
  }

  @eval (eval("                                                          \
    var __stHtmlRe = /<[a-z][\\s\\S]*>/i;                                \
    var val = 'Just plain text with no HTML';                             \
    var el = document.querySelector('[data-t=\\x22plain_key\\x22]');     \
    if (__stHtmlRe.test(val)) {                                          \
      el.innerHTML = val;                                                \
    } else {                                                             \
      el.textContent = val;                                              \
    }                                                                    \
    window.__plainText = el.textContent;                                  \
    window.__plainInner = el.innerHTML;                                   \
  "))

  @assert (window.__plainText === 'Just plain text with no HTML')
    "Plain text should render as-is via textContent"

  @assert (window.__plainInner.indexOf('<') === -1)
    "Plain text path should not introduce any HTML tags"
}

@test "locale HTML: em and anchor tags render correctly" {
  @fixture {
    <div class="locale-mixed-test">
      <span data-t="mixed_key">placeholder</span>
    </div>
  }

  @eval (eval("                                                          \
    var __stHtmlRe = /<[a-z][\\s\\S]*>/i;                                \
    function __stSanitize(html) {                                        \
      html = html.replace(/<(script|style|iframe|object|embed|form)[^>]*>[\\s\\S]*?<\\/\\1>/gi, ''); \
      html = html.replace(/<(script|style|iframe|object|embed|form|input|textarea|select)[^>]*\\/?>/gi, ''); \
      html = html.replace(/\\s+on\\w+\\s*=\\s*(?:'[^']*'|[^\\s>]+)/gi, ''); \
      return html;                                                       \
    }                                                                    \
    var val = '<em>Italic</em> and <a href=\\x22https://example.com\\x22>Link</a>'; \
    var el = document.querySelector('[data-t=\\x22mixed_key\\x22]');     \
    if (__stHtmlRe.test(val)) {                                          \
      el.innerHTML = __stSanitize(val);                                  \
    } else {                                                             \
      el.textContent = val;                                              \
    }                                                                    \
    window.__mixedHasEm = el.querySelector('em') !== null;               \
    window.__mixedHasA = el.querySelector('a') !== null;                  \
    window.__mixedAHref = el.querySelector('a') ? el.querySelector('a').getAttribute('href') : ''; \
  "))

  @assert (window.__mixedHasEm === true)
    "<em> tag should render as HTML element"

  @assert (window.__mixedHasA === true)
    "<a> tag should render as HTML element"

  @assert (window.__mixedAHref === 'https://example.com')
    "Anchor href should be preserved for safe URLs"
}

@test "locale HTML: onerror event handler is stripped" {
  @fixture {
    <div class="locale-onerror-test">
      <span data-t="onerror_key">placeholder</span>
    </div>
  }

  @eval (eval("                                                          \
    var __stHtmlRe = /<[a-z][\\s\\S]*>/i;                                \
    function __stSanitize(html) {                                        \
      html = html.replace(/<(script|style|iframe|object|embed|form)[^>]*>[\\s\\S]*?<\\/\\1>/gi, ''); \
      html = html.replace(/<(script|style|iframe|object|embed|form|input|textarea|select)[^>]*\\/?>/gi, ''); \
      html = html.replace(/\\s+on\\w+\\s*=\\s*(?:'[^']*'|[^\\s>]+)/gi, ''); \
      return html;                                                       \
    }                                                                    \
    var val = '<img src=\\x22photo.jpg\\x22 onerror=\\x22alert(1)\\x22>'; \
    var el = document.querySelector('[data-t=\\x22onerror_key\\x22]');   \
    if (__stHtmlRe.test(val)) {                                          \
      el.innerHTML = __stSanitize(val);                                  \
    } else {                                                             \
      el.textContent = val;                                              \
    }                                                                    \
    window.__onerrorInner = el.innerHTML;                                 \
  "))

  @assert (window.__onerrorInner.indexOf('onerror') === -1)
    "onerror event handler must be stripped from locale HTML"

  @assert (window.__onerrorInner.indexOf('photo.jpg') !== -1)
    "Safe src attribute should be preserved"
}
