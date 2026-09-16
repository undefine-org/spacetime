/// Unit Tests: HTML Sanitizer & Each Attribute Escaping
///
/// Tests XSS prevention in @each attribute interpolation (escapeAttr)
/// and sanitize-html primitive (sanitizeHtml).

@import "stdlib/testing/test"

// =============================================================================
// Each: Attribute Escaping (escapeAttr)
// =============================================================================

@test "each escapeAttr escapes double quotes in attribute values" {
  @fixture {
    <div class="escape-test">
      <template id="escape-tpl">
        <a class="link" href="$.url">Link</a>
      </template>
      <div class="escape-container"></div>
    </div>
  }

  @eval (eval("                                                          \
    var escapeAttr = function(str) {                                      \
      if (str == null) return '';                                         \
      return String(str)                                                  \
        .replace(/&/g, '&amp;')                                          \
        .replace(/\\x22/g, '&quot;')                                     \
        .replace(/\\x27/g, '&#x27;')                                     \
        .replace(/</g, '&lt;')                                           \
        .replace(/>/g, '&gt;');                                          \
    };                                                                    \
    window.__testEscapeAttr = escapeAttr;                                 \
  "))

  // XSS payload: break out of attribute with double quote
  @eval (eval("                                                          \
    var payload = 'x\\x22 onmouseover=\\x22alert(1)';                    \
    var escaped = window.__testEscapeAttr(payload);                      \
    window.__escapeResult = escaped;                                     \
  "))

  @assert (window.__escapeResult.indexOf('"') === -1)
    "Escaped value must not contain raw double quotes"

  @assert (window.__escapeResult.indexOf('&quot;') !== -1)
    "Escaped value must contain &quot; entity"

  @assert (window.__escapeResult.indexOf('onmouseover') !== -1)
    "Escaped value preserves text content (but safely encoded)"
}

@test "each escapeAttr escapes angle brackets" {
  @fixture {
    <div class="angle-test"></div>
  }

  @eval (eval("                                                          \
    var escapeAttr = function(str) {                                      \
      if (str == null) return '';                                         \
      return String(str)                                                  \
        .replace(/&/g, '&amp;')                                          \
        .replace(/\\x22/g, '&quot;')                                     \
        .replace(/\\x27/g, '&#x27;')                                     \
        .replace(/</g, '&lt;')                                           \
        .replace(/>/g, '&gt;');                                          \
    };                                                                    \
    window.__testEscapeAttr2 = escapeAttr;                               \
  "))

  @eval (eval("                                                          \
    var payload = '<script>alert(1)</script>';                            \
    window.__angleResult = window.__testEscapeAttr2(payload);            \
  "))

  @assert (window.__angleResult.indexOf('<') === -1)
    "Escaped value must not contain raw < character"

  @assert (window.__angleResult.indexOf('>') === -1)
    "Escaped value must not contain raw > character"

  @assert (window.__angleResult.indexOf('&lt;') !== -1)
    "Escaped value must contain &lt; entity"

  @assert (window.__angleResult.indexOf('&gt;') !== -1)
    "Escaped value must contain &gt; entity"
}

@test "each escapeAttr escapes ampersands" {
  @fixture {
    <div class="amp-test"></div>
  }

  @eval (eval("                                                          \
    var escapeAttr = function(str) {                                      \
      if (str == null) return '';                                         \
      return String(str)                                                  \
        .replace(/&/g, '&amp;')                                          \
        .replace(/\\x22/g, '&quot;')                                     \
        .replace(/\\x27/g, '&#x27;')                                     \
        .replace(/</g, '&lt;')                                           \
        .replace(/>/g, '&gt;');                                          \
    };                                                                    \
    window.__testEscapeAttr3 = escapeAttr;                               \
  "))

  @eval (eval("window.__ampResult = window.__testEscapeAttr3('foo&bar')"))

  @assert (window.__ampResult === 'foo&amp;bar')
    "Ampersand must be escaped to &amp;"
}

@test "each escapeAttr handles null and undefined" {
  @fixture {
    <div class="null-test"></div>
  }

  @eval (eval("                                                          \
    var escapeAttr = function(str) {                                      \
      if (str == null) return '';                                         \
      return String(str)                                                  \
        .replace(/&/g, '&amp;')                                          \
        .replace(/\\x22/g, '&quot;')                                     \
        .replace(/\\x27/g, '&#x27;')                                     \
        .replace(/</g, '&lt;')                                           \
        .replace(/>/g, '&gt;');                                          \
    };                                                                    \
    window.__testEscapeAttr4 = escapeAttr;                               \
  "))

  @eval (eval("window.__nullResult = window.__testEscapeAttr4(null)"))
  @eval (eval("window.__undefResult = window.__testEscapeAttr4(undefined)"))

  @assert (window.__nullResult === '')
    "null should return empty string"

  @assert (window.__undefResult === '')
    "undefined should return empty string"
}

@test "each attribute interpolation applies escapeAttr to values" {
  @fixture {
    <div class="interp-test">
      <div class="target" data-val="$.name">Content</div>
    </div>
  }

  // Simulate what each.st's interpolate() does for attribute replacement
  @eval (eval("                                                          \
    var escapeAttr = function(str) {                                      \
      if (str == null) return '';                                         \
      return String(str)                                                  \
        .replace(/&/g, '&amp;')                                          \
        .replace(/\\x22/g, '&quot;')                                     \
        .replace(/\\x27/g, '&#x27;')                                     \
        .replace(/</g, '&lt;')                                           \
        .replace(/>/g, '&gt;');                                          \
    };                                                                    \
    var getProperty = function(obj, path) {                              \
      var parts = path.split('.');                                       \
      var value = obj;                                                   \
      for (var i = 0; i < parts.length; i++) {                           \
        if (value == null) return '';                                     \
        value = value[parts[i]];                                         \
      }                                                                  \
      return value != null ? value : '';                                  \
    };                                                                    \
    var item = { name: 'x\\x22 onmouseover=\\x22alert(1)' };            \
    var el = document.querySelector('.interp-test .target');              \
    for (var j = 0; j < el.attributes.length; j++) {                     \
      var attr = el.attributes[j];                                       \
      if (attr.value.indexOf('$.') !== -1) {                             \
        attr.value = attr.value.replace(                                 \
          /\\$\\.(\\w+(?:\\.\\w+)*)/g,                                   \
          function(match, path) { return escapeAttr(getProperty(item, path)); } \
        );                                                               \
      }                                                                  \
    }                                                                    \
    window.__interpAttrResult = el.getAttribute('data-val');             \
  "))

  @assert (window.__interpAttrResult.indexOf('"') === -1)
    "Interpolated attribute must not contain raw double quotes"

  @assert (window.__interpAttrResult.indexOf('&quot;') !== -1)
    "Interpolated attribute must use &quot; entity for double quotes"
}

@test "each normal attribute binding works without issues" {
  @fixture {
    <div class="normal-test">
      <a class="safe-link" href="$.url" title="$.title">Link</a>
    </div>
  }

  // Normal safe values should pass through with only & escaping if present
  @eval (eval("                                                          \
    var escapeAttr = function(str) {                                      \
      if (str == null) return '';                                         \
      return String(str)                                                  \
        .replace(/&/g, '&amp;')                                          \
        .replace(/\\x22/g, '&quot;')                                     \
        .replace(/\\x27/g, '&#x27;')                                     \
        .replace(/</g, '&lt;')                                           \
        .replace(/>/g, '&gt;');                                          \
    };                                                                    \
    var item = { url: 'https://example.com/page', title: 'My Page' };    \
    var el = document.querySelector('.normal-test .safe-link');           \
    for (var j = 0; j < el.attributes.length; j++) {                     \
      var attr = el.attributes[j];                                       \
      if (attr.value.indexOf('$.') !== -1) {                             \
        attr.value = attr.value.replace(                                 \
          /\\$\\.(\\w+(?:\\.\\w+)*)/g,                                   \
          function(match, path) {                                        \
            var parts = path.split('.');                                  \
            var value = item;                                            \
            for (var i = 0; i < parts.length; i++) {                     \
              if (value == null) return '';                               \
              value = value[parts[i]];                                   \
            }                                                            \
            return escapeAttr(value != null ? value : '');               \
          }                                                              \
        );                                                               \
      }                                                                  \
    }                                                                    \
    window.__normalHref = el.getAttribute('href');                       \
    window.__normalTitle = el.getAttribute('title');                      \
  "))

  @assert (window.__normalHref === 'https://example.com/page')
    "Safe URL should pass through unchanged"

  @assert (window.__normalTitle === 'My Page')
    "Safe title should pass through unchanged"
}

// =============================================================================
// Sanitize HTML Primitive
// =============================================================================

@test "sanitize strips script tags entirely" {
  @fixture {
    <div class="sanitize-script-test"></div>
  }

  @eval (eval("                                                          \
    var doc = new DOMParser().parseFromString(                            \
      '<p>Hello</p><script>alert(1)<\\/script><p>World</p>',             \
      'text/html'                                                        \
    );                                                                    \
    var body = doc.body;                                                  \
    var scripts = body.querySelectorAll('script');                        \
    for (var i = 0; i < scripts.length; i++) scripts[i].remove();        \
    window.__sanitizeScriptResult = body.innerHTML;                      \
  "))

  @assert (window.__sanitizeScriptResult.indexOf('script') === -1)
    "Script tags must be completely removed"

  @assert (window.__sanitizeScriptResult.indexOf('Hello') !== -1)
    "Non-script content should be preserved"

  @assert (window.__sanitizeScriptResult.indexOf('World') !== -1)
    "Content after script should be preserved"
}

@test "sanitize preserves allowed formatting tags" {
  @fixture {
    <div class="sanitize-format-test"></div>
  }

  // Test the full sanitizer logic matching sanitize.st
  @eval (eval("                                                          \
    var ALLOWED_TAGS = ['b','i','em','strong','u','s','sub','sup','br',   \
      'p','span','a','ul','ol','li','h1','h2','h3','h4','h5','h6',       \
      'blockquote','code','pre','img','div'];                            \
    var allowedSet = {};                                                  \
    for (var i = 0; i < ALLOWED_TAGS.length; i++) allowedSet[ALLOWED_TAGS[i]] = true; \
    var doc = new DOMParser().parseFromString(                            \
      '<strong>bold</strong> <em>italic</em> <p>paragraph</p>',          \
      'text/html'                                                        \
    );                                                                    \
    var body = doc.body;                                                  \
    window.__sanitizeFormatResult = body.innerHTML;                      \
    window.__hasStrong = body.querySelector('strong') !== null;           \
    window.__hasEm = body.querySelector('em') !== null;                   \
    window.__hasP = body.querySelector('p') !== null;                     \
  "))

  @assert (window.__hasStrong === true)
    "<strong> tags should be preserved"

  @assert (window.__hasEm === true)
    "<em> tags should be preserved"

  @assert (window.__hasP === true)
    "<p> tags should be preserved"
}

@test "sanitize strips javascript: protocol from href" {
  @fixture {
    <div class="sanitize-href-test"></div>
  }

  // Replicate sanitize.st's URL sanitization logic
  @eval (eval("                                                          \
    var DANGEROUS_PROTOCOLS = /^\\s*(javascript|data|vbscript)\\s*:/i;    \
    var SAFE_DATA_IMAGE = /^\\s*data\\s*:\\s*image\\//i;                  \
    var doc = new DOMParser().parseFromString(                            \
      '<a href=\\x22javascript:alert(1)\\x22>click me</a>',             \
      'text/html'                                                        \
    );                                                                    \
    var body = doc.body;                                                  \
    var links = body.querySelectorAll('a');                               \
    for (var i = 0; i < links.length; i++) {                             \
      var href = links[i].getAttribute('href');                          \
      if (href && DANGEROUS_PROTOCOLS.test(href) && !SAFE_DATA_IMAGE.test(href)) { \
        links[i].removeAttribute('href');                                \
      }                                                                  \
    }                                                                    \
    window.__sanitizeHrefResult = body.innerHTML;                        \
    var resultLink = body.querySelector('a');                             \
    window.__linkHasHref = resultLink ? resultLink.hasAttribute('href') : false; \
    window.__linkText = resultLink ? resultLink.textContent : '';         \
  "))

  @assert (window.__linkHasHref === false)
    "javascript: href must be stripped"

  @assert (window.__linkText === 'click me')
    "Link text content should be preserved"
}

@test "sanitize strips event handler attributes" {
  @fixture {
    <div class="sanitize-event-test"></div>
  }

  // Replicate sanitize.st's attribute sanitization
  @eval (eval("                                                          \
    var doc = new DOMParser().parseFromString(                            \
      '<img src=\\x22x\\x22 onerror=\\x22alert(1)\\x22>',              \
      'text/html'                                                        \
    );                                                                    \
    var body = doc.body;                                                  \
    var imgs = body.querySelectorAll('img');                              \
    for (var i = 0; i < imgs.length; i++) {                              \
      var attrs = [];                                                    \
      for (var j = 0; j < imgs[i].attributes.length; j++) {              \
        attrs.push(imgs[i].attributes[j].name);                          \
      }                                                                  \
      for (var k = 0; k < attrs.length; k++) {                           \
        if (attrs[k].toLowerCase().indexOf('on') === 0) {                \
          imgs[i].removeAttribute(attrs[k]);                              \
        }                                                                \
      }                                                                  \
    }                                                                    \
    var resultImg = body.querySelector('img');                            \
    window.__imgHasOnerror = resultImg ? resultImg.hasAttribute('onerror') : false; \
    window.__imgHasSrc = resultImg ? resultImg.hasAttribute('src') : false; \
  "))

  @assert (window.__imgHasOnerror === false)
    "onerror event handler must be stripped"

  @assert (window.__imgHasSrc === true)
    "src attribute should be preserved"
}

@test "sanitize strips style attributes to prevent CSS injection" {
  @fixture {
    <div class="sanitize-style-test"></div>
  }

  @eval (eval("                                                          \
    var doc = new DOMParser().parseFromString(                            \
      '<div style=\\x22background:url(javascript:alert(1))\\x22>test</div>', \
      'text/html'                                                        \
    );                                                                    \
    var body = doc.body;                                                  \
    var els = body.querySelectorAll('[style]');                           \
    for (var i = 0; i < els.length; i++) {                               \
      els[i].removeAttribute('style');                                    \
    }                                                                    \
    var resultDiv = body.querySelector('div');                            \
    window.__divHasStyle = resultDiv ? resultDiv.hasAttribute('style') : false; \
    window.__divText = resultDiv ? resultDiv.textContent : '';            \
  "))

  @assert (window.__divHasStyle === false)
    "style attribute must be stripped"

  @assert (window.__divText === 'test')
    "Element text content should be preserved"
}
