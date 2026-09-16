/// Value-Change Animation Tests
///
/// Tests for the split-flap character animation behavior.
/// Verifies staying chars persist, exit/enter overlap, and final text is correct.

// --- Staying characters persist ---

@test "value-change keeps staying chars visible during animation" needs layout {
  @fixture {
    <div class="vc-test">abc</div>
  }
  @given .vc-test {
    @value-change(duration: 300ms, stagger: 30ms) {
      :entering { opacity: 0 -> 1; }
      :exiting { opacity: 1 -> 0; }
    }
  }
  // Trigger text mutation: a stays, b→x, c stays
  @wait 100ms
  @eval (eval("document.querySelector('.vc-test').textContent = 'axc'"))
  @wait 50ms
  // During animation: staying chars 'a' and 'c' are plain text nodes (prefix/suffix)
  @assert (eval("(function() { var el = document.querySelector('.vc-test'); var wrapper = el.querySelector('span'); if (!wrapper) return false; var text = wrapper.textContent; return text.indexOf('a') === 0 && text.indexOf('c') === text.length - 1; })()")) "staying chars a and c should be visible as prefix/suffix"
}

// --- Exit and enter chars coexist ---

@test "value-change animates exit and enter concurrently" needs layout {
  @fixture {
    <div class="vc-test2">hello</div>
  }
  @given .vc-test2 {
    @value-change(duration: 400ms, stagger: 40ms) {
      :entering { opacity: 0 -> 1; }
      :exiting { opacity: 1 -> 0; }
    }
  }
  // Trigger text mutation: e exits, u enters — but only AFTER the driver's
  // selector-init attached its MutationObserver (the scan is async; a
  // mutation landing before init is silently missed).
  @wait 100ms
  @eval (eval("document.querySelector('.vc-test2').textContent = 'hullo'"))
  @wait 80ms
  // Both exit span ('e') and enter span ('u') should be animating in the DOM
  @assert (eval("(function() { var el = document.querySelector('.vc-test2'); var html = el.innerHTML; return html.includes('e') && html.includes('u'); })()")) "exit and enter chars should coexist during animation"
}

// --- Final text correct after animation ---

@test "value-change restores plain text after animation completes" {
  @fixture {
    <div class="vc-test3">old</div>
  }
  @given .vc-test3 {
    @value-change(duration: 100ms, stagger: 10ms) {
      :entering { opacity: 0 -> 1; }
      :exiting { opacity: 1 -> 0; }
    }
  }
  @eval (eval("document.querySelector('.vc-test3').textContent = 'new'"))
  @wait 300ms
  @then .vc-test3 should have_text "new"
  // Should be plain text, no wrapper spans
  @assert (eval("document.querySelector('.vc-test3').querySelectorAll('span').length === 0")) "should be plain text after animation"
}

// --- No extra DOM mutations (old text restore removed) ---

@test "value-change does not flash old text before animating" {
  @fixture {
    <div class="vc-test4">first</div>
  }
  @eval (window.__vc4mutations = [])
  @eval (eval("var __obs_opts = {}; __obs_opts['childList'] = true; __obs_opts['characterData'] = true; __obs_opts['subtree'] = true; new MutationObserver(function(muts) { muts.forEach(function(m) { window.__vc4mutations.push(document.querySelector('.vc-test4').textContent); }); }).observe(document.querySelector('.vc-test4'), __obs_opts)"))
  @given .vc-test4 {
    @value-change(duration: 200ms, stagger: 20ms) {
      :entering { opacity: 0 -> 1; }
      :exiting { opacity: 1 -> 0; }
    }
  }
  @eval (eval("document.querySelector('.vc-test4').textContent = 'second'"))
  @wait 50ms
  // The mutation log should NOT contain "first" after the initial "second" set —
  // that would mean old text was restored (the bug we're fixing)
  @assert (eval("(function() { var m = window.__vc4mutations; var secondIdx = m.indexOf('second'); return secondIdx >= 0 && m.slice(secondIdx + 1).indexOf('first') === -1; })()")) "should not restore old text before animation"
}


// --- Emoji grapheme handling ---

@test "value-change handles emoji graphemes correctly" {
  @fixture {
    <div class="vc-emoji">hello</div>
  }
  @given .vc-emoji {
    @value-change(duration: 100ms, stagger: 10ms) {
      :entering { opacity: 0 -> 1; }
      :exiting { opacity: 1 -> 0; }
    }
  }
  @eval (eval("document.querySelector('.vc-emoji').textContent = '\ud83d\udc4b\ud83c\udf0d'"))
  @wait 300ms
  @then .vc-emoji should have_text "\ud83d\udc4b\ud83c\udf0d"
  @assert (eval("document.querySelector('.vc-emoji').querySelectorAll('span').length === 0")) "should be plain text after emoji animation"
}