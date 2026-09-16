// @loading pattern - Reusable loading state indicator
//
// Usage:
//   .submit-button {
//       @loading();
//   }
//
// Parameters: none

@pattern loading() {
    @state(when: "loading") {
        opacity: 0.6;
        pointer-events: none;
    }
}
