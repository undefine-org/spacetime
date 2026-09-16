/// Dev Media Picker Primitive
///
/// Adds hover overlay on images with data-st-bind, data-st-id, or data-t
/// in dev edit mode. Click opens native file picker, uploads to the dev
/// server, and updates the JSON field via EditJson WebSocket message.
///
/// Dependency: Requires dev-ws and dev-editable to be active.

%primitive dev-media-picker() {
  %emit js {
    (function() {
      var styleEl = null;
      var fileInput = null;
      var observer = null;
      var MAX_SIZE = 10 * 1024 * 1024; // 10MB upload limit

      // --- Inject overlay styles ---
      styleEl = document.createElement('style');
      styleEl.setAttribute('data-st-dev', 'media-picker');
      styleEl.textContent = [
        'html[data-st-edit-mode] img[data-st-bind], html[data-st-edit-mode] img[data-st-id], html[data-st-edit-mode] img[data-t] { position: relative; }',
        '.st-mp-wrap { position: relative; display: inline-block; }',
        '.st-mp-wrap .st-mp-overlay { position: absolute; bottom: 6px; right: 6px; width: 28px; height: 28px; background: rgba(30,30,46,0.85); border: 1px solid #313244; border-radius: 6px; display: flex; align-items: center; justify-content: center; cursor: pointer; opacity: 0; transition: opacity 0.15s ease; z-index: 9998; font-size: 14px; line-height: 1; pointer-events: auto; }',
        '.st-mp-wrap:hover .st-mp-overlay { opacity: 1; }',
        '.st-mp-overlay:hover { background: rgba(30,30,46,0.95); border-color: #89b4fa; }',
        '.st-mp-wrap .st-mp-loading { position: absolute; inset: 0; background: rgba(30,30,46,0.6); display: flex; align-items: center; justify-content: center; z-index: 9999; border-radius: 4px; }',
        '.st-mp-loading::after { content: ""; width: 20px; height: 20px; border: 2px solid #313244; border-top-color: #89b4fa; border-radius: 50%; animation: st-mp-spin 0.6s linear infinite; }',
        '@keyframes st-mp-spin { to { transform: rotate(360deg); } }'
      ].join('\n');
      if (document.head) document.head.appendChild(styleEl);

      // Hidden file input shared across all images
      fileInput = document.createElement('input');
      fileInput.type = 'file';
      fileInput.accept = 'image/*';
      fileInput.style.display = 'none';
      document.body.appendChild(fileInput);

      var pendingImg = null;
      var pendingWrap = null;

      // Check if an image is editable in dev mode
      function isEditableImage(img) {
        if (!document.documentElement.hasAttribute('data-st-edit-mode')) return false;
        return img.hasAttribute('data-st-bind') ||
               img.hasAttribute('data-st-id') ||
               img.hasAttribute('data-t');
      }

      // Wrap an image with the overlay container
      function wrapImage(img) {
        if (img.parentNode && img.parentNode.classList &&
            img.parentNode.classList.contains('st-mp-wrap')) return;
        var wrap = document.createElement('span');
        wrap.className = 'st-mp-wrap';
        wrap.style.display = 'inline-block';
        img.parentNode.insertBefore(wrap, img);
        wrap.appendChild(img);

        var overlay = document.createElement('span');
        overlay.className = 'st-mp-overlay';
        overlay.textContent = '\uD83D\uDCF7'; // camera emoji
        overlay.title = 'Replace image';
        wrap.appendChild(overlay);

        overlay.addEventListener('click', function(e) {
          e.preventDefault();
          e.stopPropagation();
          pendingImg = img;
          pendingWrap = wrap;
          fileInput.value = '';
          fileInput.click();
        });
      }

      // Unwrap an image (restore original DOM)
      function unwrapImage(img) {
        var wrap = img.parentNode;
        if (!wrap || !wrap.classList || !wrap.classList.contains('st-mp-wrap')) return;
        wrap.parentNode.insertBefore(img, wrap);
        wrap.remove();
      }

      // Scan DOM for editable images and wrap them
      function scanImages() {
        var inEditMode = document.documentElement.hasAttribute('data-st-edit-mode');
        var imgs = document.querySelectorAll('img[data-st-bind], img[data-st-id], img[data-t]');
        for (var i = 0; i < imgs.length; i++) {
          if (inEditMode) {
            wrapImage(imgs[i]);
          } else {
            unwrapImage(imgs[i]);
          }
        }
      }

      // Handle file selection
      fileInput.addEventListener('change', function() {
        if (!fileInput.files || !fileInput.files[0] || !pendingImg) return;
        var file = fileInput.files[0];

        if (file.size > MAX_SIZE) {
          console.warn('[st-dev] Image too large (max 10MB):', file.name, file.size);
          return;
        }

        var img = pendingImg;
        var wrap = pendingWrap;
        var originalSrc = img.src;

        // Show loading indicator
        var loader = document.createElement('span');
        loader.className = 'st-mp-loading';
        if (wrap) wrap.appendChild(loader);

        // Preview with object URL immediately
        var previewUrl = URL.createObjectURL(file);
        img.src = previewUrl;

        // Upload to dev server
        var formData = new FormData();
        formData.append('file', file);

        fetch('/__spacetime/dev/upload', { method: 'POST', body: formData })
          .then(function(res) {
            if (!res.ok) throw new Error('Upload failed: ' + res.status);
            return res.json();
          })
          .then(function(data) {
            // Revoke preview URL, use server path
            URL.revokeObjectURL(previewUrl);
            img.src = data.path;
            if (loader && loader.parentNode) loader.remove();

            // Persist via EditJson WebSocket message
            var ws = window.__stDevWs;
            if (!ws) return;

            var bindAttr = img.getAttribute('data-st-bind');
            var stId = img.getAttribute('data-st-id');
            var origin = img.getAttribute('data-st-origin');

            // A DATA-BOUND image (`data-st-bind="$post.cover"`) persists through the SAME
            // single-object/collection rail as text editing (W3): derive {source, path}
            // from the bind and route via dev-save, which resolves the backing file from
            // ST._dataRegistry.get(source).src (FUP-047). A data-bound <img> is NOT text-
            // bearing, so it carries NO data-st-origin — the old `bindAttr && origin`
            // guard made on-page image edits a silent no-op. Harmonized here (W3b).
            if (bindAttr && bindAttr.charAt(0) === '$' && window.__stDevSave) {
              var raw = bindAttr.replace(/^\$/, '');           // 'post.cover' / 'posts[1].cover'
              var m = raw.match(/^([^.\[]+)(.*)$/);
              if (m && m[1] && m[2]) {
                window.__stDevSave.save({
                  editType: 'data',
                  source: m[1],
                  path: m[2],                                  // '.cover' / '[1].cover'
                  value: data.path,
                  element: img
                });
                pendingImg = null; pendingWrap = null;
                return;
              }
            }

            if (bindAttr && origin) {
              // Legacy: data-st-bind="field" data-st-origin="file.json" (non-$ bind).
              var opId = 'op_' + Date.now() + '_' + Math.random().toString(36).substr(2, 6);
              ws.send({
                type: 'EditJson',
                file: origin,
                path: bindAttr,
                value: data.path,
                op_id: opId
              });
            } else if (stId && origin) {
              // data-st-id based element (.st-source image)
              var opId = 'op_' + Date.now() + '_' + Math.random().toString(36).substr(2, 6);
              ws.send({
                type: 'EditJson',
                file: origin,
                path: stId,
                value: data.path,
                op_id: opId
              });
            }
          })
          .catch(function(err) {
            console.error('[st-dev] Image upload failed:', err);
            // Revert to original source
            URL.revokeObjectURL(previewUrl);
            img.src = originalSrc;
            if (loader && loader.parentNode) loader.remove();
          });

        pendingImg = null;
        pendingWrap = null;
      });

      // Watch for DOM changes and edit mode toggle
      scanImages();
      observer = new MutationObserver(function() { scanImages(); });
      observer.observe(document.documentElement, {
        attributes: true,
        attributeFilter: ['data-st-edit-mode'],
        childList: true,
        subtree: true
      });

      %cleanup {
        if (observer) observer.disconnect();
        if (styleEl && styleEl.parentNode) styleEl.parentNode.removeChild(styleEl);
        if (fileInput && fileInput.parentNode) fileInput.parentNode.removeChild(fileInput);
        // Unwrap all wrapped images
        var wraps = document.querySelectorAll('.st-mp-wrap');
        for (var i = 0; i < wraps.length; i++) {
          var img = wraps[i].querySelector('img');
          if (img) wraps[i].parentNode.insertBefore(img, wraps[i]);
          wraps[i].remove();
        }
      }
    })();
  }
}
