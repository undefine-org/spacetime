// — Method reader shell ——————————————————————————————————————
// The method's own page is held to the method's standard (P8): legible,
// no decoration that isn't doing work.

body {
  margin: 0;
  background: #0b0d12;
  color: #c9d1de;
  font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
  font-size: 16px;
  line-height: 1.72;
}

.lit-prose, .lit-src {
  max-width: 47rem;
  margin-inline: auto;
  padding-inline: 1.4rem;
}

.lit-prose h1 {
  font-size: clamp(1.85rem, 1.2rem + 2.6vw, 2.7rem);
  line-height: 1.15;
  letter-spacing: -0.02em;
  color: #f2f5fa;
  margin: 2.6rem 0 1rem;
}

.lit-prose h2 {
  font-size: clamp(1.25rem, 1rem + 1vw, 1.5rem);
  line-height: 1.3;
  color: #f2f5fa;
  margin: 2.6rem 0 0.9rem;
  padding-top: 1.5rem;
  border-top: 1px solid #1e2530;
}

.lit-prose h3 {
  font-size: 1.06rem;
  color: #e6ebf3;
  margin: 1.9rem 0 0.6rem;
}

.lit-prose p { margin: 0 0 1rem; }

.lit-prose strong { color: #f2f5fa; font-weight: 650; }
.lit-prose em { color: #b9c4d4; }

.lit-prose a {
  color: #5eead4;
  text-decoration-color: #2a6b62;
  text-underline-offset: 0.18em;
}
.lit-prose a:hover { color: #99f6e4; }

.lit-prose code {
  font-family: ui-monospace, "SF Mono", "IBM Plex Mono", monospace;
  font-size: 0.85em;
  background: #151a23;
  color: #a5f3e4;
  padding: 0.1em 0.36em;
  border: 1px solid #212936;
}

.lit-prose pre {
  background: #070910;
  border: 1px solid #1e2530;
  padding: 0.95rem 1.1rem;
  overflow-x: auto;
  font-size: 0.83rem;
  line-height: 1.6;
}
.lit-prose pre code { background: none; border: none; padding: 0; color: #9fb0c9; }

.lit-prose ul, .lit-prose ol { margin: 0 0 1.1rem; padding-left: 1.3rem; }
.lit-prose li { margin-bottom: 0.42rem; }

.lit-prose blockquote {
  margin: 1.3rem 0;
  padding: 0.2rem 0 0.2rem 1.1rem;
  border-left: 2px solid #5eead4;
  color: #dbe3ee;
}

.lit-prose hr { border: none; border-top: 1px solid #1e2530; margin: 2.2rem 0; }

.lit-src {
  background: #070910;
  border: 1px solid #1e2530;
  padding: 0.9rem 1.1rem;
  overflow-x: auto;
  font-size: 0.8rem;
  line-height: 1.6;
  color: #8b9bb4;
  margin-bottom: 1.2rem;
}
