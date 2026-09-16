/// Video Visibility Macro
///
/// Pauses a video when it scrolls out of the viewport and resumes
/// playback when it becomes visible again.
///
/// @example
/// .hero-video {
///   @video-visibility(threshold: 0);
/// }

%macro video-visibility {

  %form {
    @video-visibility(
      threshold: $threshold:number = 0
    )
  }

  %binds {
    video-visibility(&self, threshold: $threshold)
  }
}
