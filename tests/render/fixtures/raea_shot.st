// tests/render/fixtures/raea_shot.st — a `@form shot` stamped by the runtime
// (PLAN-151). The shot's markup reaches the page through the template runtime
// (`ST.parseHtml`: `<template>.innerHTML` → `content.firstElementChild`). raea's
// DOM had neither a markup-parsing `innerHTML` nor `template.content`, so the
// shot never appeared: a solid red square must cover the frame's centre by
// the end of the score.

@version 2026-06-09;

@form shot --red-square {
  :markup {
    <div class="sq"></div>
  }
  :motion {
    opacity: 0 -> 1;
  }
}

<div class="film"><div class="slot"></div></div>

body { margin: 0; }
.film { position: fixed; inset: 0; background: #fff; display: grid; place-content: center; }
.sq { width: 160px; height: 160px; background: #f00; }
.film { @score &.time(1s) { &slot at 0s for 1s; } }
.slot { &red-square(); }
