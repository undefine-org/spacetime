// Spacetime Animation Definition
// Learn more at https://spacetime.dev

// Title fade-in on load
@version 2026-07-26;

.title {
  @on &.load(name: title-entrance, duration: 600ms, delay: 0ms, threshold: 0.1, immediate: true) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            opacity: 0 -> 1;
            translate-y: 20px -> 0;
            easing: --ease-out-quad;
        }

      }
}

// Subtitle follows title
.subtitle {
  @on &.load(name: subtitle-entrance, duration: 500ms, delay: 200ms, threshold: 0.1, immediate: true) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            opacity: 0 -> 1;
            translate-y: 15px -> 0;
            easing: --ease-out-quad;
        }

      }
}

// Button entrance
.cta-button {
  @on &.load(name: button-entrance, duration: 400ms, delay: 400ms, threshold: 0.1, immediate: true) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            opacity: 0 -> 1;
            scale: 0.9 -> 1;
            easing: --ease-out-back;
        }

      }
  @on &.hover(name: button-hover, duration: 200ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            scale: 1 -> 1.05;
            box-shadow: 0 4px 12px rgba(0,0,0,0.15) -> 0 8px 24px rgba(0,0,0,0.2);
            easing: --ease-out-quad;
        }

      }
}
