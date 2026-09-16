/// HTML Sanitizer Primitive
///
/// Provides a configurable HTML sanitizer that strips dangerous tags and attributes
/// while preserving allowed formatting elements. Uses DOMParser for safe parsing.
///
/// @yields $sanitize - Function to sanitize HTML strings

%primitive sanitize-html() {
  %emit js {
    // Allowed tags whitelist
    const ALLOWED_TAGS = new Set([
      'b', 'i', 'em', 'strong', 'u', 's', 'sub', 'sup', 'br', 'p', 'span',
      'a', 'ul', 'ol', 'li', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
      'blockquote', 'code', 'pre', 'img', 'div'
    ]);

    // Allowed attributes per tag
    const ALLOWED_ATTRS = {
      'a': new Set(['href', 'title', 'target', 'class', 'id']),
      'img': new Set(['src', 'alt', 'width', 'height', 'class', 'id']),
      '*': new Set(['class', 'id'])
    };

    // Dangerous URL protocols
    const DANGEROUS_PROTOCOLS = /^\s*(javascript|data|vbscript)\s*:/i;
    const SAFE_DATA_IMAGE = /^\s*data\s*:\s*image\//i;

    const sanitizeHtml = (html) => {
      if (!html || typeof html !== 'string') return '';

      // Parse via DOMParser (safe — doesn't execute scripts)
      const doc = new DOMParser().parseFromString(html, 'text/html');
      const body = doc.body;

      // Walk and sanitize
      const walk = (parent) => {
        const nodesToRemove = [];

        for (const node of parent.childNodes) {
          if (node.nodeType === Node.TEXT_NODE) continue;

          if (node.nodeType === Node.ELEMENT_NODE) {
            const tagName = node.tagName.toLowerCase();

            // Remove forbidden tags entirely (including contents for script/style)
            if (tagName === 'script' || tagName === 'style' || tagName === 'iframe' ||
                tagName === 'object' || tagName === 'embed' || tagName === 'form' ||
                tagName === 'input' || tagName === 'textarea' || tagName === 'select') {
              nodesToRemove.push(node);
              continue;
            }

            // For non-whitelisted tags, keep text content but remove the tag
            if (!ALLOWED_TAGS.has(tagName)) {
              // Replace with children
              while (node.firstChild) {
                parent.insertBefore(node.firstChild, node);
              }
              nodesToRemove.push(node);
              continue;
            }

            // Sanitize attributes
            const allowedForTag = ALLOWED_ATTRS[tagName] || ALLOWED_ATTRS['*'];
            const globalAllowed = ALLOWED_ATTRS['*'];

            for (const attr of Array.from(node.attributes)) {
              const attrName = attr.name.toLowerCase();

              // Remove ALL event handlers
              if (attrName.startsWith('on')) {
                node.removeAttribute(attr.name);
                continue;
              }

              // Remove style attribute (CSS injection vector)
              if (attrName === 'style') {
                node.removeAttribute(attr.name);
                continue;
              }

              // Check if attribute is allowed
              if (!allowedForTag.has(attrName) && !globalAllowed.has(attrName)) {
                node.removeAttribute(attr.name);
                continue;
              }

              // Sanitize URL attributes
              if (attrName === 'href' || attrName === 'src' || attrName === 'action') {
                const val = attr.value;
                if (DANGEROUS_PROTOCOLS.test(val) && !SAFE_DATA_IMAGE.test(val)) {
                  node.removeAttribute(attr.name);
                }
              }
            }

            // Recurse into children
            walk(node);
          } else {
            // Remove comments and other non-element/text nodes
            nodesToRemove.push(node);
          }
        }

        for (const node of nodesToRemove) {
          node.remove();
        }
      };

      walk(body);
      return body.innerHTML;
    };

    %yield sanitizeHtml -> $sanitize;

    %cleanup {}
  }

  %exports {
    $sanitize: fn(string) ~> string
  }
}
