/// Unit Tests: Color Module (OKLCH-first)
///
/// Tests color parsing and interpolation with OKLCH as the primary color space.

@import "stdlib/testing/test"

// =============================================================================
// Color Parsing - Hex
// =============================================================================

@test "parseColor handles 3-digit hex" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.color = ST.parseColor('#f80')"))

  @assert (color !== null) "Should parse 3-digit hex"
  @assert (color.l > 0 && color.l <= 1) "Lightness should be normalized"
  @assert (color.a === 1) "Alpha should default to 1"
}

@test "parseColor handles 6-digit hex" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.color = ST.parseColor('#ff8800')"))

  @assert (color !== null) "Should parse 6-digit hex"
  @assert (Math.abs(color.l - ST.parseColor('#f80').l) < 0.01)
    "Should match 3-digit equivalent"
}

@test "parseColor handles 8-digit hex with alpha" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.color = ST.parseColor('#ff880080')"))

  @assert (color !== null) "Should parse 8-digit hex"
  @assert (Math.abs(color.a - 0.5) < 0.02) "Alpha should be ~0.5"
}

// =============================================================================
// Color Parsing - OKLCH (Native)
// =============================================================================

@test "parseColor handles oklch()" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.color = ST.parseColor('oklch(70% 0.15 30)')"))

  @assert (color !== null) "Should parse oklch()"
  @assert (Math.abs(color.l - 0.7) < 0.01) "Lightness should be 0.7"
  @assert (Math.abs(color.c - 0.15) < 0.01) "Chroma should be 0.15"
  @assert (Math.abs(color.h - 30) < 0.1) "Hue should be 30"
}

@test "parseColor handles oklch() with alpha" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.color = ST.parseColor('oklch(70% 0.15 30 / 0.5)')"))

  @assert (color !== null) "Should parse oklch() with alpha"
  @assert (Math.abs(color.a - 0.5) < 0.01) "Alpha should be 0.5"
}

// =============================================================================
// Color Parsing - Edge Cases
// =============================================================================

@test "parseColor returns null for invalid input" {
  @fixture { <div class="test"></div> }

  @assert (ST.parseColor(null) === null) "null should return null"
  @assert (ST.parseColor(undefined) === null) "undefined should return null"
  @assert (ST.parseColor('notacolor') === null) "invalid string"
}

@test "isColor helper works" {
  @fixture { <div class="test"></div> }

  @assert (ST.isColor('#ff0000') === true) "Hex should be detected"
  @assert (ST.isColor('hello') === false) "Text should not be color"
  @assert (ST.isColor('100px') === false) "Length should not be color"
}

// =============================================================================
// Color Interpolation
// =============================================================================

@test "interpolateColor interpolates in OKLCH space" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.red = ST.parseColor('#ff0000')"))
  @eval (eval("window.blue = ST.parseColor('#0000ff')"))
  @eval (eval("window.mid = ST.interpolateColor(red, blue, 0.5)"))

  @assert (mid !== null) "Should return interpolated color"
  @assert (mid.startsWith('oklch(')) "Should return oklch() format"
}

@test "interpolateColor at boundaries" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.c1 = ST.parseColor('#ff0000')"))
  @eval (eval("window.c2 = ST.parseColor('#00ff00')"))

  @eval (eval("window.atStart = ST.interpolateColor(c1, c2, 0)"))
  @eval (eval("window.atEnd = ST.interpolateColor(c1, c2, 1)"))

  @assert (atStart !== null) "Should return color at t=0"
  @assert (atEnd !== null) "Should return color at t=1"
}

// =============================================================================
// RGB Fallback
// =============================================================================

@test "interpolateColorRgb returns rgb() format" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.c1 = ST.parseColor('#ff0000')"))
  @eval (eval("window.c2 = ST.parseColor('#0000ff')"))
  @eval (eval("window.result = ST.interpolateColorRgb(c1, c2, 0.5)"))

  @assert (result.startsWith('rgb')) "Should return rgb format"
}

// =============================================================================
// Color Conversion Round-Trip
// =============================================================================

@test "hex -> OKLCH -> RGB round trip" {
  @fixture { <div class="test"></div> }

  @let (original = '#ff8800')
  @let (oklch = ST.parseColor(original))
  @eval (eval("window.__back = ST._colorUtils.oklchToRgb(oklch)"))

  @assert (Math.abs(__back.r - 255) < 5) "Red should be close"
  @assert (Math.abs(__back.g - 136) < 5) "Green should be close"
  @assert (Math.abs(__back.b - 0) < 5) "Blue should be close"
}
