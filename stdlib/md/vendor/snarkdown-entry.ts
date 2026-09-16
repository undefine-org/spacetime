// Vendored-entry wrapper for snarkdown (mirrors stdlib/text/vendor/pretext-entry.ts).
//
// bun's IIFE format drops ES exports. This wrapper imports snarkdown's default
// export from the pinned submodule and assigns it onto `globalThis.snarkdown`,
// so the bundled IIFE exposes a stable global our `%emit js` primitives can call
// as `snarkdown(markdownText) -> htmlString`. Keep in sync with the `exports:`
// field of the `%vendor snarkdown` declaration in MODULE.st / vendor.st.
import parse from "./snarkdown/src/index.js";

(globalThis as any).snarkdown = parse;
