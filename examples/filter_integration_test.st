// Integration test for filter properties
// This file tests that multiple filters combine correctly in a single animation


.test-card {
    @on &.hover(name: test-hover, duration: 300ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            blur: 0px -> 5px;
            brightness: 1 -> 1.2;
            contrast: 1 -> 1.1;
            translate-y: 0 -> -10px;
            scale: 1 -> 1.05;
            opacity: 1 -> 0.95;
            easing: --ease-out-expo;
        }

      }
}
