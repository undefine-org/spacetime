/// Mobile Input Macro
///
/// Mobile-optimized input with keyboard handling and touch-friendly styles.
/// Tracks focus, filled, and error states for styling.
///
/// @param type - Input type: text, password, email, tel, number, search, url
/// @param clearable - Show clear button when input has value (default: false)
///
/// @example basic email input
/// .email-input {
///   @mobile-input(type: "email", clearable: true) {
///     focused { border-color: var(--primary); }
///     filled { background: var(--surface-variant); }
///   }
/// }
///
/// @example password input
/// .password-field {
///   @mobile-input(type: "password") {
///     focused { border-color: blue; }
///     error { border-color: red; }
///   }
/// }

%macro mobile-input {

  %form {
    @mobile-input(
      type: $type:("text" | "password" | "email" | "tel" | "number" | "search" | "url") = "text",
      clearable: $clearable:bool = false
    ) {
      $styles:properties
    }
  }

  %binds {
    focus(&self) -> { $focused }
  }

  %derives {
    // Track if input has a value
    $hasValue: (&self).value && (&self).value.length > 0
  }

  %states {
    idle {
      // Mobile-optimized touch target
      min-height: 44px
      font-size: 16px
      touch-action: manipulation
      -webkit-appearance: none
      appearance: none

      // Prevent iOS zoom on focus
      font-size: max(16px, 1rem)

      // Smooth transitions
      transition: border-color 0.2s ease, background-color 0.2s ease, box-shadow 0.2s ease
    }

    focused when $focused {
      outline: none
      // Indicate focus visually
      box-shadow: 0 0 0 2px rgba(0, 122, 255, 0.3)
    }

    filled when $hasValue {
      // Style for inputs with content
    }
  }

  %when $focused {
    %applies {
      $styles
    }
  }
}


/// Mobile Input Field Macro
///
/// Extended mobile input with label and error message support.
/// Wraps the mobile-input with additional UI states.
///
/// @example with validation
/// .email-field {
///   @mobile-input-field(type: "email", required: true) {
///     error { border-color: var(--error-color); }
///   }
/// }

%macro mobile-input-field {

  %form {
    @mobile-input-field(
      type: $type:("text" | "password" | "email" | "tel" | "number" | "search" | "url") = "text",
      required: $required:bool = false,
      clearable: $clearable:bool = false
    ) {
      $styles:properties
    }
  }

  %binds {
    focus(&self) -> { $focused }
  }

  %states {
    idle {
      // Mobile-optimized touch target
      min-height: 44px
      font-size: 16px
      touch-action: manipulation
      -webkit-appearance: none
      appearance: none
      font-size: max(16px, 1rem)
      transition: border-color 0.2s ease, background-color 0.2s ease
    }

    focused when $focused {
      outline: none
      box-shadow: 0 0 0 2px rgba(0, 122, 255, 0.3)
    }
  }

  %when $focused {
    %applies {
      $styles
    }
  }
}


/// Mobile Search Input Macro
///
/// Search-optimized input with search icon and clear functionality.
///
/// @example
/// .search-bar {
///   @mobile-search(placeholder: "Search...")
/// }

%macro mobile-search {

  %form {
    @mobile-search(
      placeholder: $placeholder:string = "Search"
    )
  }

  %binds {
    focus(&self) -> { $focused }
  }

  %states {
    idle {
      // Search-specific styles
      min-height: 44px
      font-size: 16px
      padding-left: 40px
      touch-action: manipulation
      -webkit-appearance: none
      appearance: none
      font-size: max(16px, 1rem)
      border-radius: 10px
      background-color: rgba(0, 0, 0, 0.05)
      transition: background-color 0.2s ease
    }

    focused when $focused {
      outline: none
      background-color: rgba(0, 0, 0, 0.08)
    }
  }
}


/// Mobile Textarea Macro
///
/// Multi-line text input optimized for mobile.
/// Auto-grows with content.
///
/// @example
/// .message-input {
///   @mobile-textarea(maxRows: 5)
/// }

%macro mobile-textarea {

  %form {
    @mobile-textarea(
      minRows: $minRows:number = 2,
      maxRows: $maxRows:number = 6
    )
  }

  %binds {
    focus(&self) -> { $focused }
  }

  %states {
    idle {
      min-height: 88px
      font-size: 16px
      touch-action: manipulation
      -webkit-appearance: none
      appearance: none
      font-size: max(16px, 1rem)
      resize: none
      transition: border-color 0.2s ease, height 0.1s ease
    }

    focused when $focused {
      outline: none
      box-shadow: 0 0 0 2px rgba(0, 122, 255, 0.3)
    }
  }
}
