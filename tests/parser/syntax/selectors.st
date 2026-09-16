// CSS Selector syntax reference file
// Shows all selector variants supported in Spacetime

// ============================================
// Simple Selectors
// ============================================

// Element selector
// div { }
// button { }
// input { }
// span { }
// article { }

// Class selector
// .card { }
// .btn-primary { }
// .my-component { }
// .active { }

// ID selector
// #header { }
// #main-content { }
// #sidebar { }

// Universal selector
// * { }

// ============================================
// Attribute Selectors
// ============================================

// Has attribute
// [disabled] { }
// [data-active] { }
// [hidden] { }

// Exact match
// [type="submit"] { }
// [data-state="open"] { }
// [role="button"] { }

// Contains word
// [class~="featured"] { }

// Starts with (prefix)
// [href^="https"] { }
// [class^="btn-"] { }

// Ends with (suffix)
// [src$=".png"] { }
// [href$=".pdf"] { }

// Contains substring
// [class*="btn"] { }
// [data-id*="item"] { }

// Dash match (hyphen-separated)
// [lang|="en"] { }

// ============================================
// Pseudo-classes
// ============================================

// State pseudo-classes
// :hover { }
// :active { }
// :focus { }
// :focus-visible { }
// :focus-within { }
// :disabled { }
// :enabled { }
// :checked { }
// :visited { }
// :link { }

// Structural pseudo-classes
// :first-child { }
// :last-child { }
// :only-child { }
// :first-of-type { }
// :last-of-type { }
// :only-of-type { }

// Functional pseudo-classes with arguments
// :nth-child(2) { }
// :nth-child(odd) { }
// :nth-child(even) { }
// :nth-child(2n+1) { }
// :nth-child(3n) { }
// :nth-last-child(2) { }
// :nth-of-type(2n+1) { }
// :nth-last-of-type(3) { }

// Negation pseudo-class
// :not(.hidden) { }
// :not([disabled]) { }
// :not(:first-child) { }

// Other functional pseudo-classes
// :is(.btn, .button) { }
// :where(.card, .panel) { }
// :has(> img) { }

// Empty and content pseudo-classes
// :empty { }
// :root { }

// ============================================
// Pseudo-elements
// ============================================

// Content pseudo-elements
// ::before { }
// ::after { }

// Selection pseudo-elements
// ::selection { }
// ::placeholder { }

// Form pseudo-elements
// ::first-line { }
// ::first-letter { }

// Webkit-specific (with single colon fallback)
// ::-webkit-scrollbar { }

// ============================================
// Compound Selectors
// ============================================

// Element + class
// button.primary { }
// div.card.featured { }
// a.link.external { }

// Element + ID
// div#main { }
// section#hero { }

// Element + attribute
// input[type="text"] { }
// button[disabled] { }
// a[href^="https"] { }

// Element + pseudo-class
// button:hover { }
// a:visited { }
// input:focus { }
// li:first-child { }

// Element + pseudo-element
// p::first-line { }
// a::after { }

// Class + attribute
// .btn[type="submit"] { }

// Class + pseudo-class
// .card:hover { }
// .item:nth-child(odd) { }

// Multiple classes
// .btn.primary.large { }
// .card.featured.active { }

// ID + class
// #header.fixed { }

// Multiple modifiers combined
// button.primary:hover[disabled] { }
// input.form-control:focus[required] { }

// ============================================
// Combinators
// ============================================

// Descendant combinator (space)
// .card .title { }
// #nav a { }
// article p span { }

// Child combinator (>)
// .list > li { }
// nav > ul > li { }
// .card > .header { }

// Adjacent sibling combinator (+)
// h1 + p { }
// .label + .input { }
// dt + dd { }

// General sibling combinator (~)
// h1 ~ p { }
// .toggle ~ .content { }
// input ~ .error { }

// ============================================
// Selector Lists
// ============================================

// Multiple selectors (comma-separated)
// h1, h2, h3 { }
// .btn, .button, [role="button"] { }
// article, section, aside { }
// .card, #panel, [data-component] { }

// ============================================
// Complex Selectors
// ============================================

// Nested descendant with pseudo
// .modal > .content .header:first-child { }

// Deep nesting with multiple combinators
// nav > ul > li > a:hover::after { }

// Mixed combinators
// .sidebar > nav ul li a { }

// Complex with negation
// [data-state="active"].card:not(.hidden) { }

// Multiple compound selectors with combinators
// .form-group > .label + .input:focus { }

// Complex selector lists
// .btn:hover, .btn:focus, .btn:active { }

// Self-reference in Spacetime contexts
// & { }
// &:hover { }
// &.active { }
// & > .child { }
// &::before { }

// ============================================
// Spacetime-specific Selector Usage
// ============================================

// In scope blocks
// .component {
//     background: #fff;
// }

// Nested scopes
// .parent {
//     > .child {
//         color: blue;
//     }
// }

// With element references
// &button .icon { }

// Attribute selector variations
// [data-*="value"] { }
// [slot="header"] { }
// [part="button"] { }

// ============================================
// Edge Cases
// ============================================

// Selector starting with combinator (in nesting context)
// > .direct-child { }
// + .adjacent { }
// ~ .sibling { }

// Unicode class names (if supported)
// .item-1 { }
// .-webkit-appearance { }

// Escaped characters in selectors
// .\:hover { }
// .item\[0\] { }
