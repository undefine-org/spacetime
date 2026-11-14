/// Reference tokens. The launch film's palette and public X VF brand typography
/// are intentional; change these once to reskin every scene and component.
@font("Launch Sans") { src: url("assets/fonts/XVF.woff2") format("woff2"); font-weight: 100 900; font-display: block; }
@font("Launch Serif") { src: url("assets/fonts/Tinos.ttf") format("truetype"); font-weight: 400; font-display: block; }
:root {
  --film-paper: #fdfdfd;
  --film-ink: #101111;
  --film-blue: #0f83fd;
  --film-mint: #00bba5;
  --film-orange: #ff7900;
  --film-red: #fd243b;
  --film-purple: #773de8;
  --film-cloud: #e9f4fc;
  --film-peach: #fcf1e8;
  --film-pink: #fbeaec;
  --film-muted: #73777b;
  --film-ui: #f4f5f4;
  --film-bubble: #eaf0f5;
  --brand-display: "Launch Sans", Arial, sans-serif;
  --brand-editorial: "Launch Serif", "Times New Roman", serif;
}
html, body { margin: 0; width: 100%; height: 100%; overflow: hidden; background: #e8e8e8; }
body { color: var(--film-ink); font-family: var(--brand-display); -webkit-font-smoothing: antialiased; }
* { box-sizing: border-box; }
.film { position: fixed; inset: 0; overflow: hidden; container-type: size; }
.film__canvas {
  position: absolute; left: 50%; top: 50%; width: 1920px; height: 1080px;
  translate: -50% -50%;
  scale: min(calc(100cqw / 1920px), calc(100cqh / 1080px));
  overflow: hidden; background: var(--film-paper);
}
.scene { position: absolute; inset: 0; opacity: 0; overflow: hidden; background: var(--film-paper); }
.scene__camera { position: absolute; inset: 0; transform-origin: center; }
.scene--blue { background: var(--film-cloud); }
.scene--peach { background: var(--film-peach); }
.scene--pink { background: var(--film-pink); }
.center { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; }
.title { margin: 0; font-size: 88px; line-height: 1.12; font-weight: 400; letter-spacing: -3.2px; white-space: nowrap; }
.caption { margin: 0; font-size: 88px; line-height: 1.1; font-weight: 400; letter-spacing: -3px; }
.editorial { font-family: var(--brand-editorial); font-weight: 400; letter-spacing: -2px; }
