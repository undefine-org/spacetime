/// Spacetime Mobile Typography
///
/// Mobile-optimized type scale with font sizes, line heights, and weights.
/// Uses CSS custom properties for easy theming and consistency.
///
/// @example
/// .container {
///   @mobile-typography
/// }
///
/// .heading {
///   font-size: var(--st-text-2xl);
///   font-weight: var(--st-font-bold);
///   line-height: var(--st-leading-tight);
/// }
///
/// .body {
///   font-size: var(--st-text-base);
///   font-weight: var(--st-font-normal);
///   line-height: var(--st-leading-normal);
/// }

%macro mobile-typography {

  %form {
    @mobile-typography
  }

  %states {
    base {
      // Font sizes - mobile optimized scale
      --st-text-xs: 12px
      --st-text-sm: 14px
      --st-text-base: 16px
      --st-text-lg: 18px
      --st-text-xl: 20px
      --st-text-2xl: 24px
      --st-text-3xl: 30px

      // Line heights
      --st-leading-tight: 1.25
      --st-leading-normal: 1.5
      --st-leading-relaxed: 1.75

      // Font weights
      --st-font-normal: 400
      --st-font-medium: 500
      --st-font-semibold: 600
      --st-font-bold: 700
    }
  }
}


/// Heading Typography Macro
///
/// Applies typography settings for headings.
/// Provides common heading sizes with appropriate weights and line heights.
///
/// @param size - Heading size level (1-6)
///
/// @example
/// .page-title {
///   @mobile-heading(size: 1)
/// }

%macro mobile-heading {

  %form {
    @mobile-heading(
      size: $size:number = 1
    )
  }

  %derives {
    $isH1: $size == 1
    $isH2: $size == 2
    $isH3: $size == 3
    $isH4: $size == 4
    $isH5: $size == 5
    $isH6: $size == 6
  }

  %states {
    h1 when $isH1 {
      font-size: var(--st-text-3xl, 30px)
      font-weight: var(--st-font-bold, 700)
      line-height: var(--st-leading-tight, 1.25)
    }

    h2 when $isH2 {
      font-size: var(--st-text-2xl, 24px)
      font-weight: var(--st-font-bold, 700)
      line-height: var(--st-leading-tight, 1.25)
    }

    h3 when $isH3 {
      font-size: var(--st-text-xl, 20px)
      font-weight: var(--st-font-semibold, 600)
      line-height: var(--st-leading-tight, 1.25)
    }

    h4 when $isH4 {
      font-size: var(--st-text-lg, 18px)
      font-weight: var(--st-font-semibold, 600)
      line-height: var(--st-leading-normal, 1.5)
    }

    h5 when $isH5 {
      font-size: var(--st-text-base, 16px)
      font-weight: var(--st-font-medium, 500)
      line-height: var(--st-leading-normal, 1.5)
    }

    h6 when $isH6 {
      font-size: var(--st-text-sm, 14px)
      font-weight: var(--st-font-medium, 500)
      line-height: var(--st-leading-normal, 1.5)
    }
  }
}


/// Body Text Typography Macro
///
/// Applies typography settings for body text.
///
/// @param size - Text size (xs, sm, base, lg)
///
/// @example
/// .paragraph {
///   @mobile-text(size: "base")
/// }
///
/// .caption {
///   @mobile-text(size: "xs")
/// }

%macro mobile-text {

  %form {
    @mobile-text(
      size: $size:("xs" | "sm" | "base" | "lg") = "base"
    )
  }

  %derives {
    $isXs: $size == "xs"
    $isSm: $size == "sm"
    $isBase: $size == "base"
    $isLg: $size == "lg"
  }

  %states {
    text-xs when $isXs {
      font-size: var(--st-text-xs, 12px)
      font-weight: var(--st-font-normal, 400)
      line-height: var(--st-leading-normal, 1.5)
    }

    text-sm when $isSm {
      font-size: var(--st-text-sm, 14px)
      font-weight: var(--st-font-normal, 400)
      line-height: var(--st-leading-normal, 1.5)
    }

    text-base when $isBase {
      font-size: var(--st-text-base, 16px)
      font-weight: var(--st-font-normal, 400)
      line-height: var(--st-leading-relaxed, 1.75)
    }

    text-lg when $isLg {
      font-size: var(--st-text-lg, 18px)
      font-weight: var(--st-font-normal, 400)
      line-height: var(--st-leading-relaxed, 1.75)
    }
  }
}
