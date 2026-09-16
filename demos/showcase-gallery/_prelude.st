/// Showcase gallery — the chrome AROUND the specimens.
///
/// Deliberately quiet: every pattern on this page brings its own design
/// language, and five of them are loud. The gallery's job is to disappear —
/// neutral ground, small type, generous rules — so the only thing competing
/// for attention is the work.
///
/// The specimen frames are declared here as forms so the page body stays a
/// list of patterns rather than a pile of layout.

@form style --gal-ground {
    background: #ffffff;
    color: #14141a;
    font-family: system-ui, -apple-system, "Segoe UI", sans-serif;
    -webkit-font-smoothing: antialiased;
}

@form style --gal-mono {
    font-family: ui-monospace, "SF Mono", "JetBrains Mono", Menlo, monospace;
    font-size: 12px;
    letter-spacing: 0.02em;
}

/// The label strip above each specimen. Small, uppercase, unmistakably
/// chrome — it must never read as part of the pattern below it.
@form style --gal-eyebrow {
    --gal-mono;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    font-size: 11px;
    color: #6b6b7b;
}

@form style --gal-rule {
    border: 0;
    border-top: 1px solid #e6e6ee;
}

/// A specimen is a page-sized design shown whole.
///
/// Patterns declare `min-height: 100vh` and are composed for a real viewport,
/// so a fixed-height frame with `overflow: hidden` shows their TOP-LEFT CORNER
/// and nothing else — a 112px display headline crops to three letters. The
/// frame therefore holds a full-size STAGE that is scaled down, the way a
/// device preview works: the pattern still lays out at 1440x900, and the
/// viewer sees all of it.
@form style --gal-frame {
    position: relative;
    width: 100%;
    /* The stage lays out at 1440px and scales to the frame's own width
       (1122/1440 = 0.779), so the preview fills the column edge to edge with
       no dead strip. The tallest specimen is ~1120px real, which lands at
       ~873px — sized to that so every pattern is shown WHOLE, since a frame
       shorter than its content is exactly the crop this replaced. */
    height: 878px;
    overflow: hidden;
    border: 1px solid #e6e6ee;
    border-radius: 14px;
    background: #fbfbfd;
}

/// The stage lays out at a real desktop viewport, then scales to fit the
/// frame. `transform-origin: top left` anchors the shrink so the design's own
/// top-left stays put; the scale is the frame width (1072px of the 1120px
/// column) over the stage width.
@form style --gal-stage {
    position: absolute;
    top: 0;
    left: 0;
    width: 1440px;
    transform: scale(0.779);
    transform-origin: top left;
}

@form style --gal-chip {
    --gal-mono;
    display: inline-block;
    padding: 6px 12px;
    border: 1px solid #d8d8e4;
    border-radius: 999px;
    background: #ffffff;
    color: #3a3a48;
    cursor: pointer;
    user-select: none;
}

@form style --gal-chip-on {
    background: #14141a;
    border-color: #14141a;
    color: #ffffff;
}
