%primitive brace-regex-test() {
  %emit js {
    (function() {
      var styleRe = /:root\s*\{[^}]*\}/g;
      var simpleRe = /a\{b\}/;
      window.__brace_regex_test = { hasStyleRe: !!styleRe, hasSimple: !!simpleRe };
    })();
  }
}

body {
  @brace-regex-test()
}
