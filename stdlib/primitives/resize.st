/// Resize Observer Primitive
///
/// Wraps the ResizeObserver API to detect element size changes.
/// Useful for responsive layouts and size-dependent animations.
///
/// @param &el - The element to observe
/// @param box - Box model to observe: "content-box", "border-box", "device-pixel-content-box"
///
/// @yields $width - Content width in pixels
/// @yields $height - Content height in pixels
/// @yields $rect - Full DOMRectReadOnly
/// @yields $blockSize - Block size (height in horizontal writing mode)
/// @yields $inlineSize - Inline size (width in horizontal writing mode)

%primitive resize(
  &el,
  box: ("content-box" | "border-box" | "device-pixel-content-box") = "content-box"
) {
  %emit js {
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];

      // Content rect (always available)
      const rect = entry.contentRect;
      %yield rect.width -> $width;
      %yield rect.height -> $height;
      %yield rect -> $rect;

      // Box-specific sizes (may not be available in all browsers)
      let boxSize;
      switch ('%box') {
        case "border-box":
          boxSize = entry.borderBoxSize?.[0];
          break;
        case "device-pixel-content-box":
          boxSize = entry.devicePixelContentBoxSize?.[0];
          break;
        default:
          boxSize = entry.contentBoxSize?.[0];
      }

      if (boxSize) {
        %yield boxSize.blockSize -> $blockSize;
        %yield boxSize.inlineSize -> $inlineSize;
      } else {
        // Fallback for older browsers
        %yield rect.height -> $blockSize;
        %yield rect.width -> $inlineSize;
      }

      // Aspect ratio
      %yield rect.width / rect.height -> $aspectRatio;
    });

    observer.observe(%&el, { box: '%box' });

    %cleanup {
      observer.disconnect();
    }
  }

  %exports {
    $width: number        // content width
    $height: number       // content height
    $rect: DOMRectReadOnly
    $blockSize: number    // block size (height in LTR)
    $inlineSize: number   // inline size (width in LTR)
    $aspectRatio: number  // width / height
  }
}
