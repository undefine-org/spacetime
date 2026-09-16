// @value-change Demo
// Per-character flip animations when text content changes
// Features: diff mode, stagger directions, logarithmic speed-up

// Basic counter with stagger from start (left-to-right cascade)
@version 2026-07-26;

.counter {
    @on &.text-change(duration: 300ms, stagger: 40ms, from: start) {
        :entering {
          translate-y: -100% -> 0
          opacity: 0 -> 1
          easing: cubic-bezier(0, 0, 0.2, 1)
        }
        :exiting {
          translate-y: 0 -> 100%
          opacity: 1 -> 0
          easing: cubic-bezier(0.4, 0, 1, 1)
        }
      }
}

// Price display with stagger from end (right-to-left cascade)
// Good for prices where the significant digits are on the left
.price {
    @on &.text-change(duration: 400ms, stagger: 50ms, from: end) {
        :entering {
          translate-y: -100% -> 0
          opacity: 0 -> 1
        }
        :exiting {
          translate-y: 0 -> 100%
          opacity: 1 -> 0.5 -> 0
        }
      }
}

// Quantity selector with stagger from center (outward ripple)
.quantity {
    @on &.text-change(duration: 250ms, stagger: 30ms, from: center) {
        :entering {
          translate-y: -50% -> 0
          opacity: 0 -> 1
          easing: cubic-bezier(0, 0, 0.2, 1)
        }
        :exiting {
          translate-y: 0 -> 50%
          opacity: 1 -> 0
          easing: cubic-bezier(0.4, 0, 0.6, 1)
        }
      }
}

// Score display with horizontal slide animation
.score {
    @on &.text-change(duration: 350ms, stagger: 35ms, from: start) {
        :entering {
          translate-x: 100% -> 0
          opacity: 0 -> 1
        }
        :exiting {
          translate-x: 0 -> -100%
          opacity: 1 -> 0
        }
      }
}

// Timer with scale effect
.timer {
    @on &.text-change(duration: 200ms, stagger: 30ms, from: start) {
        :entering {
          scale: 1.5 -> 1
          opacity: 0 -> 1
        }
        :exiting {
          scale: 1 -> 0.5
          opacity: 1 -> 0
        }
      }
}

// Combined rotation and translation for creative effect
.odometer {
    @on &.text-change(duration: 500ms, stagger: 60ms, from: start) {
        :entering {
          translate-y: -120% -> 0
          rotate-x: 90deg -> 0deg
          opacity: 0 -> 1
        }
        :exiting {
          translate-y: 0 -> 120%
          rotate-x: 0deg -> -90deg
          opacity: 1 -> 0
        }
      }
}
