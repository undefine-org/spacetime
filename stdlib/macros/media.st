/// Video Visibility Macro
///
/// Pauses a video when it scrolls out of the viewport and resumes
/// playback when it becomes visible again.

%macro video-visibility {

  %form {
    @video-visibility(
      /// Intersection threshold (0-1)
      threshold: $threshold:number = 0
    )
  }

  %binds {
    video-visibility(&self, threshold: $threshold)
  }

  %example "@video-visibility" {
    .hero-video {
      @video-visibility(threshold: 0);
    }
  }
}
