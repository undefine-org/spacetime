// Demonstration of the new value declaration syntax


.counter {
    // Initialized value declarations
    $count number: 0;
    $message string: "Click me";
    $enabled boolean: true;

    // Uninitialized (optional) value declaration
    $error string?;

    @on click increment {
        scale: 1 -> 1.05 -> 1;
    }
}

.product-list {
    // Array types
    $items Product[]: [];
    $selectedIndex number: 0;

    @on &.scroll(name: reveal, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: 40px -> 0;
      }
}

.greeting {
    // String value with quotes
    $welcomeMessage string: "Hello, Spacetime!";
    $isVisible boolean: false;

    @on &.load(name: fadeIn, duration: 800ms, delay: 0ms, threshold: 0.1, immediate: false) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
      }
}
