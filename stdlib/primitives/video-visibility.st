/// Video Visibility Primitive
///
/// Automatically pauses a video when it scrolls out of the viewport
/// and resumes playback when it becomes visible again.
/// Uses IntersectionObserver for efficient detection.

%primitive video-visibility(
  /// The video element to control
  &el,
  threshold: number = 0
) {
  %emit js {
    const el = %&el;
    const threshold = %threshold;

    if (!el || el.tagName !== 'VIDEO') {
      console.warn('[ST] video-visibility: element is not a <video>');
      return;
    }

    const observer = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          el.play().catch(() => {});
        } else {
          el.pause();
        }
      }
    }, { threshold: threshold });

    observer.observe(el);

    %cleanup {
      observer.disconnect();
    }
  }
}
