// Vendored-entry wrapper (PLAN-024 W3).
//
// bun's IIFE format drops ES exports, and bun 1.3 has no --global-name flag.
// This wrapper imports pretext's public surface from the pinned submodule and
// assigns it onto `globalThis.pretext`, so the bundled IIFE exposes a stable
// global that our %emit js primitives reference. Keep the import list in sync
// with the `exports:` field of the %vendor declaration in MODULE.st.
import {
  prepare,
  prepareWithSegments,
  layout,
  layoutWithLines,
  walkLineRanges,
  measureNaturalWidth,
} from "./pretext/src/layout.ts";

(globalThis as any).pretext = {
  prepare,
  prepareWithSegments,
  layout,
  layoutWithLines,
  walkLineRanges,
  measureNaturalWidth,
};
