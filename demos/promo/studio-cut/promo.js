// Spacetime promo — "studio" cut. One WebGL scene; every visual is a pure
// function of absolute time `t` so frames are deterministic under CDP capture.
// window.__seek(t, frame) renders exactly one frame at t seconds.
import * as THREE from 'three';
import { EffectComposer } from 'three/addons/postprocessing/EffectComposer.js';
import { RenderPass } from 'three/addons/postprocessing/RenderPass.js';
import { UnrealBloomPass } from 'three/addons/postprocessing/UnrealBloomPass.js';
import { ShaderPass } from 'three/addons/postprocessing/ShaderPass.js';
import { OutputPass } from 'three/addons/postprocessing/OutputPass.js';

const W = 1920, H = 1080;
const BG = '#06070a', INK = '#e6ecf5', DIM = '#8a94a6', AMBER = '#ffb454', CYAN = '#7dd3fc', PANEL = '#0b0e14';

// ---------------------------------------------------------------- math
const clamp = (x, a = 0, b = 1) => Math.min(b, Math.max(a, x));
const lerp = (a, b, t) => a + (b - a) * t;
const span = (t, a, b) => clamp((t - a) / (b - a));
const E = {
  linear: (x) => x,
  outExpo: (x) => (x >= 1 ? 1 : 1 - Math.pow(2, -10 * x)),
  inExpo: (x) => (x <= 0 ? 0 : Math.pow(2, 10 * x - 10)),
  inOutExpo: (x) => (x <= 0 ? 0 : x >= 1 ? 1 : x < 0.5 ? Math.pow(2, 20 * x - 10) / 2 : (2 - Math.pow(2, -20 * x + 10)) / 2),
  outCubic: (x) => 1 - Math.pow(1 - x, 3),
  inOutCubic: (x) => (x < 0.5 ? 4 * x * x * x : 1 - Math.pow(-2 * x + 2, 3) / 2),
  outQuart: (x) => 1 - Math.pow(1 - x, 4),
  inOutQuart: (x) => (x < 0.5 ? 8 * x * x * x * x : 1 - Math.pow(-2 * x + 2, 4) / 2),
  outBack: (x) => { const c1 = 1.70158, c3 = c1 + 1; return 1 + c3 * Math.pow(x - 1, 3) + c1 * Math.pow(x - 1, 2); },
  smooth: (x) => x * x * (3 - 2 * x),
};
const ev = (t, a, b, f = E.outExpo) => f(span(t, a, b));
const decay = (t, t0, tau) => (t < t0 ? 0 : Math.exp(-(t - t0) / tau));
function rng(seed) { let a = seed >>> 0; return () => { a |= 0; a = (a + 0x6d2b79f5) | 0; let t = Math.imul(a ^ (a >>> 15), 1 | a); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; }; }
// cheap deterministic 1D noise for camera shake
const n1 = (x) => Math.sin(x * 12.9898) * 0.5 + Math.sin(x * 78.233 + 1.3) * 0.35 + Math.sin(x * 37.719 + 2.1) * 0.15;

// ---------------------------------------------------------------- hits (audio-locked)
const HITS = [
  { t: 2.2, amp: 0.6 }, { t: 6.4, amp: 0.7 }, { t: 11.6, amp: 0.7 },
  { t: 18.0, amp: 1.0 }, { t: 23.0, amp: 1.1 }, { t: 27.0, amp: 1.2 },
];
const hitEnv = (t, tau) => HITS.reduce((s, h) => s + decay(t, h.t, tau) * h.amp, 0);

// ---------------------------------------------------------------- renderer
const renderer = new THREE.WebGLRenderer({ antialias: true, preserveDrawingBuffer: true, powerPreference: 'high-performance' });
renderer.setPixelRatio(1);
renderer.setSize(W, H, false);
renderer.domElement.style.width = W + 'px';
renderer.domElement.style.height = H + 'px';
renderer.toneMapping = THREE.ACESFilmicToneMapping;
renderer.toneMappingExposure = 1.0;
renderer.outputColorSpace = THREE.SRGBColorSpace;
document.body.appendChild(renderer.domElement);

const scene = new THREE.Scene();
scene.background = new THREE.Color(BG);
const camera = new THREE.PerspectiveCamera(38, W / H, 0.1, 200);
scene.add(camera);
const hud = new THREE.Group(); // camera-relative stage for flat compositions
camera.add(hud);

// ---------------------------------------------------------------- canvas plane helper
const PX = 0.0042; // world units per texture pixel at hud depth (z=-10); view is ~12.2 units wide there
const PLANES = []; // every CanvasPlane — reset to hidden at the top of each seek
class CanvasPlane {
  constructor(w, h, { additive = false } = {}) {
    this.canvas = document.createElement('canvas');
    this.canvas.width = w; this.canvas.height = h;
    this.ctx = this.canvas.getContext('2d');
    this.tex = new THREE.CanvasTexture(this.canvas);
    this.tex.colorSpace = THREE.SRGBColorSpace;
    this.tex.anisotropy = 8;
    this.tex.minFilter = THREE.LinearMipmapLinearFilter;
    this.mat = new THREE.MeshBasicMaterial({ map: this.tex, transparent: true, depthWrite: false, toneMapped: false, blending: additive ? THREE.AdditiveBlending : THREE.NormalBlending });
    this.mesh = new THREE.Mesh(new THREE.PlaneGeometry(w * PX, h * PX), this.mat);
    this.mesh.visible = false;
    this.key = '';
    PLANES.push(this);
  }
  alpha(a) { this.mat.opacity = clamp(a); this.mesh.visible = a > 0.002; return this; }
  at(x, y, z = -9.9) { this.mesh.position.set(x, y, z); return this; }
  // redraw only when the drawing inputs changed
  draw(key, fn) { if (key === this.key) return; this.key = key; const { ctx, canvas } = this; ctx.clearRect(0, 0, canvas.width, canvas.height); ctx.save(); fn(ctx, canvas.width, canvas.height); ctx.restore(); this.tex.needsUpdate = true; }
}

function text(ctx, str, { x, y, font, color = INK, spacing = 0, blur = 0, align = 'center', glow = 0, baseline = 'middle', fill = null }) {
  ctx.save();
  ctx.font = font; ctx.textAlign = align; ctx.textBaseline = baseline; ctx.letterSpacing = spacing + 'px';
  ctx.filter = blur > 0.3 ? `blur(${blur.toFixed(1)}px)` : 'none';
  if (glow > 0) { ctx.shadowColor = color; ctx.shadowBlur = glow; }
  ctx.fillStyle = fill ?? color;
  ctx.fillText(str, x, y);
  ctx.restore();
}
function rrect(ctx, x, y, w, h, r) { ctx.beginPath(); ctx.roundRect(x, y, w, h, r); }

// Title plane: big Syne display line, focus-pull in, optional shimmer.
class Title extends CanvasPlane {
  constructor() { super(3600, 700); }
  show(str, { a = 1, blur = 0, spacing = 0, size = 190, glow = 0, shimmer = null, color = INK, font = null } = {}) {
    const key = [str, blur.toFixed(1), spacing.toFixed(1), size, glow, shimmer?.toFixed(2), color].join('|');
    this.draw(key, (ctx, w, h) => {
      let fill = null;
      if (shimmer != null) {
        const g = ctx.createLinearGradient(w * (shimmer - 0.35), 0, w * (shimmer + 0.35), 0);
        g.addColorStop(0, color); g.addColorStop(0.42, color); g.addColorStop(0.5, '#fffaf0'); g.addColorStop(0.58, color); g.addColorStop(1, color); fill = g;
      }
      text(ctx, str, { x: w / 2 + spacing / 2, y: h / 2, font: font ?? `800 ${size}px Syne`, color, spacing, blur, glow, fill });
    });
    this.alpha(a);
    return this;
  }
}

// ---------------------------------------------------------------- shared 2D components
// The product card from the README. p = reveal progress 0..1 (opacity + rise).
function drawCard(ctx, x, y, w, h, p, { accent = AMBER } = {}) {
  const e = E.outExpo(p);
  ctx.save();
  ctx.globalAlpha *= e;
  ctx.translate(0, (1 - e) * h * 0.12);
  rrect(ctx, x, y, w, h, h * 0.05); ctx.fillStyle = '#0f131a'; ctx.fill();
  ctx.strokeStyle = 'rgba(255,255,255,0.14)'; ctx.lineWidth = 2; ctx.stroke();
  // image area
  const ih = h * 0.56;
  rrect(ctx, x + 2, y + 2, w - 4, ih, h * 0.05);
  const g = ctx.createLinearGradient(x, y, x + w, y + ih);
  g.addColorStop(0, '#1a2230'); g.addColorStop(1, '#0e1a22');
  ctx.fillStyle = g; ctx.fill();
  // a "mug" silhouette: circle + handle
  ctx.fillStyle = 'rgba(255,180,84,0.85)';
  ctx.beginPath(); ctx.ellipse(x + w * 0.5, y + ih * 0.55, w * 0.13, ih * 0.28, 0, 0, Math.PI * 2); ctx.fill();
  ctx.strokeStyle = 'rgba(255,180,84,0.85)'; ctx.lineWidth = w * 0.02;
  ctx.beginPath(); ctx.arc(x + w * 0.64, y + ih * 0.55, ih * 0.14, -Math.PI / 2, Math.PI / 2); ctx.stroke();
  // text
  const s = h * 0.09;
  text(ctx, 'Ceramic Mug', { x: x + w * 0.07, y: y + ih + h * 0.14, font: `600 ${s}px "Space Grotesk"`, align: 'left' });
  text(ctx, '$24.99', { x: x + w * 0.93, y: y + ih + h * 0.14, font: `500 ${s}px "Space Grotesk"`, align: 'right', color: accent });
  text(ctx, 'hand-thrown · 12 oz', { x: x + w * 0.07, y: y + ih + h * 0.30, font: `400 ${s * 0.72}px "Space Grotesk"`, align: 'left', color: DIM });
  ctx.restore();
}

// Syntax-lit Spacetime code, typed to nChars.
const TOK = /(\/\/.*$)|(@[\w-]+)|(&[\w.-]+|--[\w-]+)|(-?\d*\.?\d+(?:px|ms|s|%|em)?)|(->|=>)|([{}();:])|("[^"]*")|(\.[\w-]+)|(\s+)|([^\s{}();:]+)/gm;
const TOKCOL = [DIM, AMBER, CYAN, '#ffd9a8', AMBER, '#5c6677', '#b8f0c6', '#c9d3e0', null, '#dbe3ee', INK];
function drawCode(ctx, lines, nChars, { x, y, size = 46, lineHeight = 1.55, cursor = true }) {
  ctx.save();
  ctx.font = `500 ${size}px "JetBrains Mono"`; ctx.textBaseline = 'top'; ctx.textAlign = 'left';
  let left = nChars;
  let cx = x, cy = y;
  for (const line of lines) {
    cx = x;
    const shown = line.slice(0, Math.max(0, left));
    for (const m of shown.matchAll(TOK)) {
      const gi = m.slice(1).findIndex((g) => g !== undefined);
      ctx.fillStyle = TOKCOL[gi] ?? INK;
      ctx.fillText(m[0], cx, cy);
      cx += ctx.measureText(m[0]).width;
    }
    left -= line.length + 1;
    if (left < 0) break;
    cy += size * lineHeight;
  }
  if (cursor && left <= 0) { ctx.fillStyle = AMBER; ctx.fillRect(cx + 4, cy, size * 0.55, size * 1.1); }
  ctx.restore();
}

// ---------------------------------------------------------------- S1: dust + orb
const dustN = 2200, dustR = rng(11);
const dustPos = new Float32Array(dustN * 3), dustDir = new Float32Array(dustN * 3), dustSeed = new Float32Array(dustN);
for (let i = 0; i < dustN; i++) {
  dustPos.set([(dustR() - 0.5) * 24, (dustR() - 0.5) * 14, (dustR() - 0.5) * 16 - 2], i * 3);
  dustDir.set([(dustR() - 0.5), (dustR() - 0.5) * 0.6, (dustR() - 0.5)], i * 3);
  dustSeed[i] = dustR();
}
const dustGeo = new THREE.BufferGeometry();
dustGeo.setAttribute('position', new THREE.BufferAttribute(dustPos, 3));
dustGeo.setAttribute('dir', new THREE.BufferAttribute(dustDir, 3));
dustGeo.setAttribute('seed', new THREE.BufferAttribute(dustSeed, 1));
const dustMat = new THREE.ShaderMaterial({
  uniforms: { uT: { value: 0 }, uA: { value: 0 }, uColor: { value: new THREE.Color(INK) } },
  transparent: true, depthWrite: false, blending: THREE.AdditiveBlending,
  vertexShader: `attribute vec3 dir; attribute float seed; uniform float uT; varying float vA;
    void main(){ vec3 p = position + dir * uT * 0.12 + vec3(sin(uT*0.7+seed*6.28), cos(uT*0.5+seed*3.1), 0.0)*0.25;
      vec4 mv = modelViewMatrix * vec4(p,1.0); gl_PointSize = (1.0 + seed*2.2) * (26.0 / -mv.z); vA = 0.35 + 0.65*seed; gl_Position = projectionMatrix*mv; }`,
  fragmentShader: `uniform vec3 uColor; uniform float uA; varying float vA; void main(){ vec2 c = gl_PointCoord-0.5; float d = length(c); if(d>0.5) discard; gl_FragColor = vec4(uColor, smoothstep(0.5,0.1,d)*vA*uA*0.32); }`,
});
scene.add(new THREE.Points(dustGeo, dustMat));

function radialSprite(inner, outer) {
  const c = document.createElement('canvas'); c.width = c.height = 512;
  const g = c.getContext('2d'); const grad = g.createRadialGradient(256, 256, 0, 256, 256, 256);
  grad.addColorStop(0, inner); grad.addColorStop(0.25, inner.replace(/[\d.]+\)$/, '0.55)')); grad.addColorStop(1, outer);
  g.fillStyle = grad; g.fillRect(0, 0, 512, 512);
  const t = new THREE.CanvasTexture(c); t.colorSpace = THREE.SRGBColorSpace; return t;
}
const orbGlow = new THREE.Sprite(new THREE.SpriteMaterial({ map: radialSprite('rgba(255,200,120,1)', 'rgba(255,180,84,0)'), transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, toneMapped: false }));
scene.add(orbGlow);
const orbCore = new THREE.Mesh(new THREE.SphereGeometry(0.06, 24, 24), new THREE.MeshBasicMaterial({ color: new THREE.Color(AMBER).multiplyScalar(6), toneMapped: false }));
scene.add(orbCore);

// ---------------------------------------------------------------- S2: grid floor + time axis
const gridMat = new THREE.ShaderMaterial({
  uniforms: { uReveal: { value: 0 }, uRipple: { value: -1 }, uAxisX: { value: -100 }, uAxisGlow: { value: 0 }, uA: { value: 0 }, uColor: { value: new THREE.Color(CYAN) }, uAmber: { value: new THREE.Color(AMBER) } },
  transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, side: THREE.DoubleSide,
  vertexShader: `uniform float uRipple; varying vec3 vP;
    void main(){ vec3 p = position; float d = length(p.xy);
      if (uRipple >= 0.0) { float ring = d - uRipple*7.0; p.z += sin(ring*2.2) * exp(-ring*ring*0.08) * exp(-uRipple*1.6) * 0.6; }
      vP = p; gl_Position = projectionMatrix * modelViewMatrix * vec4(p,1.0); }`,
  fragmentShader: `uniform float uReveal, uAxisX, uAxisGlow, uA; uniform vec3 uColor, uAmber; varying vec3 vP;
    void main(){ vec2 g = abs(fract(vP.xy - 0.5) - 0.5) / fwidth(vP.xy); float line = 1.0 - min(min(g.x, g.y), 1.0);
      float d = length(vP.xy); float reveal = 1.0 - smoothstep(uReveal - 1.5, uReveal + 0.5, d);
      float fog = exp(-d * 0.055); float ax = exp(-abs(vP.x - uAxisX) * 0.9) * uAxisGlow;
      vec3 col = mix(uColor * 0.55, uAmber * 1.8, clamp(ax, 0.0, 1.0)) * (0.6 + ax * 2.0);
      float ht = clamp(vP.z * 1.5, 0.0, 1.0); col += uAmber * ht * 1.5;
      gl_FragColor = vec4(col, (line * 0.9 + ht * 0.35) * reveal * fog * uA); }`,
});
const grid = new THREE.Mesh(new THREE.PlaneGeometry(64, 64, 128, 128), gridMat);
grid.rotation.x = -Math.PI / 2;
scene.add(grid);
const axisCore = new THREE.Mesh(new THREE.PlaneGeometry(0.05, 9), new THREE.MeshBasicMaterial({ color: new THREE.Color(AMBER).multiplyScalar(4), toneMapped: false, transparent: true, depthWrite: false }));
const axisHalo = new THREE.Mesh(new THREE.PlaneGeometry(1.2, 9), new THREE.MeshBasicMaterial({ color: AMBER, toneMapped: false, transparent: true, opacity: 0.18, depthWrite: false, blending: THREE.AdditiveBlending }));
axisCore.position.y = axisHalo.position.y = 4.5; axisCore.visible = axisHalo.visible = false;
scene.add(axisCore, axisHalo);

// ---------------------------------------------------------------- hud planes
const title = new Title(); hud.add(title.mesh);
const sub = new CanvasPlane(2560, 320); hud.add(sub.mesh);
const kicker = new CanvasPlane(2560, 200); hud.add(kicker.mesh);

// S3: code panel + live card
const code = new CanvasPlane(1380, 1020); hud.add(code.mesh);
code.mesh.rotation.y = 0.16;
const CODE_LINES = [
  '.card {',
  '  @on &.visible {',
  '    opacity: 0 -> 1;',
  '    translate-y: 40px -> 0;',
  '    easing: --ease-out-expo;',
  '  }',
  '}',
];
const CODE_TOTAL = CODE_LINES.reduce((n, l) => n + l.length + 1, 0);
const live = new CanvasPlane(1180, 1020); hud.add(live.mesh);
live.mesh.rotation.y = -0.16;

// S4: four driver panels
const DRIVERS = [
  { label: '&.time(1.2s)', kind: 'Transport', x: -4.3 },
  { label: '&.scroll(cover)', kind: 'Progress', x: -1.43 },
  { label: '&.click', kind: 'Event', x: 1.43 },
  { label: '&voice.playback', kind: 'Transport', x: 4.3 },
];
const panels = DRIVERS.map(() => { const p = new CanvasPlane(1000, 840); hud.add(p.mesh); return p; });

// S5: filmstrip (drawn once)
const STRIP_N = 24, FW = 480, FH = 720;
const strip = new CanvasPlane(STRIP_N * FW, FH); hud.add(strip.mesh);
strip.draw('strip', (ctx) => {
  for (let k = 0; k < STRIP_N; k++) {
    const x = k * FW;
    ctx.fillStyle = '#0a0c11'; ctx.fillRect(x, 0, FW, FH);
    ctx.fillStyle = 'rgba(255,255,255,0.06)'; ctx.fillRect(x + FW - 2, 0, 2, FH);
    // sprockets
    ctx.fillStyle = '#06070a';
    for (let s = 0; s < 4; s++) { rrect(ctx, x + 40 + s * 110, 26, 60, 40, 8); ctx.fill(); rrect(ctx, x + 40 + s * 110, FH - 66, 60, 40, 8); ctx.fill(); }
    // frame content
    ctx.fillStyle = BG; ctx.fillRect(x + 24, 96, FW - 48, FH - 192);
    drawCard(ctx, x + 84, 150, FW - 168, FH - 300, k / (STRIP_N - 6));
    text(ctx, String(k * 75).padStart(4, '0'), { x: x + FW - 30, y: FH - 46, font: '500 26px "JetBrains Mono"', color: DIM, align: 'right' });
  }
});
const strip2 = new CanvasPlane(STRIP_N * FW, FH); hud.add(strip2.mesh);
strip2.draw('strip2', (ctx) => { ctx.drawImage(strip.canvas, 0, 0); });
strip2.mat.opacity = 0.35;
const counter = new CanvasPlane(1400, 200); hud.add(counter.mesh);
const cmd = new CanvasPlane(1800, 260); hud.add(cmd.mesh);

// S6: <script> → particles
const scriptPlane = new CanvasPlane(2048, 640); hud.add(scriptPlane.mesh);
scriptPlane.draw('script', (ctx, w, h) => text(ctx, '<script>', { x: w / 2, y: h / 2, font: '700 300px "JetBrains Mono"', color: INK }));
const shatter = (() => {
  const c = scriptPlane.canvas, g = scriptPlane.ctx;
  const img = g.getImageData(0, 0, c.width, c.height).data;
  const step = 5, pts = [], r = rng(7);
  for (let y = 0; y < c.height; y += step) for (let x = 0; x < c.width; x += step) {
    if (img[(y * c.width + x) * 4 + 3] > 128) pts.push([(x - c.width / 2) * PX, (c.height / 2 - y) * PX]);
  }
  const n = pts.length, pos = new Float32Array(n * 3), vel = new Float32Array(n * 3), seed = new Float32Array(n);
  pts.forEach(([x, y], i) => {
    pos.set([x, y, 0], i * 3);
    const a = Math.atan2(y, x * 0.35) + (r() - 0.5) * 1.2, sp = 1.5 + r() * 4.5;
    vel.set([Math.cos(a) * sp + x * 0.9, Math.sin(a) * sp + y * 1.5 + 1.2, (r() - 0.5) * 6], i * 3);
    seed[i] = r();
  });
  const geo = new THREE.BufferGeometry();
  geo.setAttribute('position', new THREE.BufferAttribute(pos, 3));
  geo.setAttribute('vel', new THREE.BufferAttribute(vel, 3));
  geo.setAttribute('seed', new THREE.BufferAttribute(seed, 1));
  const mat = new THREE.ShaderMaterial({
    uniforms: { uTau: { value: -1 }, uColor: { value: new THREE.Color(INK) }, uHot: { value: new THREE.Color(AMBER).multiplyScalar(1.6) } },
    transparent: true, depthWrite: false, blending: THREE.AdditiveBlending,
    vertexShader: `attribute vec3 vel; attribute float seed; uniform float uTau; varying float vA; varying float vS;
      void main(){ float tau = max(uTau, 0.0); float k = (1.0 - exp(-2.2*tau)) / 2.2;
        vec3 p = position + vel * k + vec3(0.0, -2.4*tau*tau, 0.0) + vec3(sin(seed*6.28+tau*4.0), cos(seed*9.0+tau*3.0), 0.0) * 0.25 * tau;
        vA = 1.0 - smoothstep(1.2, 3.4, tau); vS = seed;
        vec4 mv = modelViewMatrix * vec4(p,1.0); gl_PointSize = (3.6 + seed*3.0) * (10.0 / -mv.z) * (1.0 - 0.4*smoothstep(0.0,3.0,tau)); gl_Position = projectionMatrix*mv; }`,
    fragmentShader: `uniform vec3 uColor, uHot; uniform float uTau; varying float vA; varying float vS;
      void main(){ vec2 c = gl_PointCoord-0.5; float d=length(c); if(d>0.5) discard;
        vec3 col = mix(uColor, uHot, smoothstep(0.0,0.3,uTau)*vS*0.8); gl_FragColor = vec4(col, smoothstep(0.5,0.12,d)*vA*0.8); }`,
  });
  const p = new THREE.Points(geo, mat); p.position.z = -10; p.visible = false; hud.add(p); return p;
})();

// S7: wordmark + underline
const mark = new Title(); hud.add(mark.mesh);
const rule = new THREE.Mesh(new THREE.PlaneGeometry(1, 0.02), new THREE.MeshBasicMaterial({ color: new THREE.Color(CYAN).multiplyScalar(3), toneMapped: false, transparent: true, depthWrite: false }));
rule.visible = false; hud.add(rule);

// ---------------------------------------------------------------- post
const composer = new EffectComposer(renderer);
composer.setPixelRatio(1); composer.setSize(W, H);
composer.addPass(new RenderPass(scene, camera));
const bloom = new UnrealBloomPass(new THREE.Vector2(W, H), 0.45, 0.55, 0.86);
composer.addPass(bloom);
composer.addPass(new OutputPass());
const post = new ShaderPass({
  uniforms: { tDiffuse: { value: null }, uAb: { value: 0.002 }, uVig: { value: 0.55 }, uGrain: { value: 0.035 }, uSeed: { value: 0 }, uFlash: { value: 0 }, uGlitch: { value: 0 }, uFade: { value: 0 } },
  vertexShader: `varying vec2 vUv; void main(){ vUv = uv; gl_Position = projectionMatrix * modelViewMatrix * vec4(position,1.0); }`,
  fragmentShader: `uniform sampler2D tDiffuse; uniform float uAb, uVig, uGrain, uSeed, uFlash, uGlitch, uFade; varying vec2 vUv;
    float hash(vec2 p){ return fract(sin(dot(p, vec2(12.9898,78.233))) * 43758.5453); }
    void main(){ vec2 uv = vUv;
      if (uGlitch > 0.0) { float s = floor(uv.y * 28.0); float r = hash(vec2(s, uSeed)); if (r > 0.55) uv.x += (hash(vec2(s*3.1, uSeed+7.0)) - 0.5) * 0.09 * uGlitch; }
      vec2 d = (uv - 0.5) * uAb * (1.0 + uGlitch * 6.0);
      vec3 col = vec3(texture2D(tDiffuse, uv + d).r, texture2D(tDiffuse, uv).g, texture2D(tDiffuse, uv - d).b);
      float v = 1.0 - uVig * smoothstep(0.35, 1.25, length((uv - 0.5) * vec2(1.55, 1.0)));
      col *= v;
      col += (hash(uv * vec2(1920.0, 1080.0) + uSeed) - 0.5) * uGrain;
      col += uFlash;
      col *= 1.0 - uFade;
      gl_FragColor = vec4(col, 1.0); }`,
});
composer.addPass(post);

// ---------------------------------------------------------------- the score
const V = new THREE.Vector3(), TGT = new THREE.Vector3();
function seek(t, frame = Math.round(t * 60)) {
  // everything starts hidden; each scene shows what it owns
  for (const p of PLANES) p.alpha(0);
  rule.visible = shatter.visible = axisCore.visible = axisHalo.visible = false;
  // ---- camera path (world scenes S1–S2, then parked for hud scenes)
  let cx = 0, cy = 0.6, cz = 10, tx = 0, ty = 0, tz = 0;
  if (t < 3) { cz = lerp(10.5, 9.4, ev(t, 0, 3, E.inOutCubic)); }
  else if (t < 4.4) { const k = ev(t, 3, 4.4, E.inOutCubic); cx = 0; cy = lerp(0.6, 9.5, k); cz = lerp(9.4, 4.5, k); }
  else if (t < 5.8) { const k = ev(t, 4.4, 5.8, E.inOutCubic); cy = lerp(9.5, 2.1, k); cz = lerp(4.5, 9.5, k); ty = lerp(0, 0.9, k); }
  else { const k = ev(t, 5.8, 8.0, E.inOutCubic); cy = lerp(2.1, 1.4, k); cz = lerp(9.5, 8.6, k); ty = 0.9; }
  const sh = hitEnv(t, 0.22) * 0.09;
  camera.position.set(cx + n1(t * 60) * sh, cy + n1(t * 60 + 40) * sh, cz);
  camera.lookAt(tx, ty, tz);
  camera.rotation.z = n1(t * 60 + 80) * sh * 0.25;

  // ---- S1: point of light, dust, first line
  const s1 = t < 3.6;
  dustMat.uniforms.uT.value = t;
  dustMat.uniforms.uA.value = ev(t, 0.2, 1.6, E.smooth) * (1 - ev(t, 6.8, 7.6, E.smooth));
  const orbA = ev(t, 0.0, 0.8, E.outCubic) * (1 - ev(t, 3.2, 3.8, E.smooth));
  const flare = decay(t, 2.2, 0.35);
  orbGlow.scale.setScalar(lerp(0.3, 1.5, ev(t, 0.6, 2.2, E.outCubic)) + flare * 2.6);
  orbGlow.material.opacity = orbA * (0.75 + flare * 0.25);
  orbCore.scale.setScalar(lerp(0.2, 1, ev(t, 0.6, 1.6)) + flare * 1.5);
  orbCore.visible = orbGlow.visible = orbA > 0.01;
  if (s1) {
    const k = ev(t, 0.9, 2.3);
    const out = ev(t, 2.9, 3.5, E.smooth);
    title.at(0, 0.2).show('CSS describes space.', { a: k * (1 - out), size: 150, blur: lerp(26, 0, k) + out * 16, spacing: lerp(50, 3, k), glow: flare * 30 });
  }

  // ---- S2: the grid materializes, the time axis sweeps through
  gridMat.uniforms.uA.value = ev(t, 3.0, 3.6, E.smooth) * (1 - ev(t, 7.0, 7.8, E.smooth));
  gridMat.uniforms.uReveal.value = lerp(0, 34, ev(t, 3.0, 5.4, E.outQuart));
  gridMat.uniforms.uRipple.value = t >= 6.4 ? t - 6.4 : -1;
  const axK = span(t, 5.2, 6.35);
  const axX = lerp(-20, 20, E.inOutCubic(axK));
  gridMat.uniforms.uAxisX.value = axX;
  gridMat.uniforms.uAxisGlow.value = (t > 5.2 && t < 6.5 ? 1 : 0) * (1 - ev(t, 6.35, 6.5));
  axisCore.visible = axisHalo.visible = t > 5.2 && t < 6.5;
  axisCore.position.x = axisHalo.position.x = axX;
  axisCore.material.opacity = axisHalo.material.opacity = 0.9 * (1 - ev(t, 6.3, 6.5));
  if (t >= 3.6 && t < 7.6) {
    const k = ev(t, 4.5, 5.6), out = ev(t, 6.9, 7.5, E.smooth), hit = decay(t, 6.4, 0.3);
    title.at(0, -1.55).show('Spacetime adds time.', { a: k * (1 - out), size: 150, blur: lerp(20, 0, k) + out * 14, spacing: lerp(40, 2, k), glow: hit * 30 });
    kicker.at(0, -2.55).alpha(ev(t, 5.0, 5.8) * (1 - out)).draw('k2', (ctx, w, h) => text(ctx, 'DIMENSIONS, PLURAL', { x: w / 2, y: h / 2, font: '600 44px "Space Grotesk"', color: DIM, spacing: 14 }));
  }

  // ---- S3: code types; the described element animates live beside it
  const s3 = t >= 7.2 && t < 12.6;
  const s3in = ev(t, 7.3, 8.2), s3out = ev(t, 12.0, 12.6, E.smooth);
  if (s3) {
    const nChars = Math.floor(lerp(0, CODE_TOTAL, span(t, 7.6, 11.3)));
    const blink = Math.floor(t * 2.5) % 2 === 0;
    code.at(-2.95, 0.2 + (1 - s3in) * 0.4).alpha(s3in * (1 - s3out)).draw('code' + nChars + blink, (ctx, w, h) => {
      rrect(ctx, 0, 0, w, h, 28); ctx.fillStyle = 'rgba(11,14,20,0.92)'; ctx.fill(); ctx.strokeStyle = 'rgba(255,255,255,0.12)'; ctx.lineWidth = 2; ctx.stroke();
      ctx.fillStyle = 'rgba(255,255,255,0.05)'; ctx.fillRect(0, 0, w, 84);
      text(ctx, 'site.st', { x: 48, y: 42, font: '500 30px "JetBrains Mono"', color: DIM, align: 'left' });
      ['#ff5f57', '#febc2e', '#28c840'].forEach((c, i) => { ctx.fillStyle = c; ctx.beginPath(); ctx.arc(w - 60 - i * 40, 42, 10, 0, 7); ctx.fill(); });
      drawCode(ctx, CODE_LINES, nChars, { x: 56, y: 130, size: 62, cursor: blink });
    });
    // the card reveals exactly when the `translate-y` line has been typed
    const typedThrough = CODE_LINES.slice(0, 4).reduce((n, l) => n + l.length + 1, 0);
    const revealAt = 7.6 + (typedThrough / CODE_TOTAL) * (11.3 - 7.6);
    const p = ev(t, revealAt, revealAt + 0.9, E.linear);
    live.at(2.55, 0.2 + (1 - s3in) * 0.4).alpha(s3in * (1 - s3out)).draw('live' + p.toFixed(3), (ctx, w, h) => {
      rrect(ctx, 0, 0, w, h, 28); ctx.fillStyle = 'rgba(11,14,20,0.6)'; ctx.fill(); ctx.strokeStyle = 'rgba(255,255,255,0.08)'; ctx.lineWidth = 2; ctx.stroke();
      text(ctx, 'browser', { x: 48, y: 42, font: '500 30px "JetBrains Mono"', color: DIM, align: 'left' });
      text(ctx, '&.visible', { x: w - 48, y: 42, font: '500 30px "JetBrains Mono"', color: p > 0 ? AMBER : DIM, align: 'right' });
      drawCard(ctx, 120, 150, w - 240, 600, p);
      // playhead
      ctx.fillStyle = 'rgba(255,255,255,0.1)'; ctx.fillRect(120, h - 90, w - 240, 6);
      ctx.fillStyle = AMBER; ctx.fillRect(120, h - 90, (w - 240) * E.outExpo(p), 6);
    });
    const k = ev(t, 11.6, 12.3), hit = decay(t, 11.6, 0.3);
    title.at(0, -2.55).show('Declare it once.', { a: k * (1 - s3out), size: 140, blur: lerp(18, 0, k), spacing: lerp(30, 2, k), glow: hit * 40 });
  }

  // ---- S4: four drivers, one composition; then they converge
  const s4 = t >= 12.4 && t < 18.6;
  if (s4) {
    const conv = ev(t, 16.5, 18.0, E.inOutQuart);
    const s4in = ev(t, 12.4, 13.0), s4out = ev(t, 18.15, 18.6, E.smooth);
    DRIVERS.forEach((d, i) => {
      const P = 12.8 + i;
      const lit = t >= P;
      let p, head, ph = 0;
      if (i === 0) { p = span(t, P, P + 1.2); head = p; }
      else if (i === 1) { head = E.inOutCubic(span(t, P, P + 1.7)); p = head; }
      else if (i === 2) { p = E.outBack(span(t, P, P + 0.55)); head = lit ? 1 : 0; }
      else { p = span(t, P, P + 1.4); head = p; ph = 1; }
      const pulse = decay(t, P, 0.25);
      const pl = panels[i];
      pl.mesh.position.set(lerp(d.x, 0, conv), lerp(-0.1, 0.1, conv), -10 + i * 0.001);
      pl.mesh.rotation.y = lerp(-d.x * 0.06, 0, conv);
      pl.mesh.scale.setScalar(lerp(0.7, 1.3, conv));
      pl.alpha(s4in * (1 - s4out)).draw(`p${i}|${p.toFixed(3)}|${head.toFixed(3)}|${pulse.toFixed(2)}|${conv.toFixed(3)}`, (ctx, w, h) => {
        rrect(ctx, 2, 2, w - 4, h - 4, 26); ctx.fillStyle = PANEL; ctx.fill();
        ctx.strokeStyle = lit ? `rgba(255,180,84,${0.5 + pulse * 0.5})` : 'rgba(255,255,255,0.12)'; ctx.lineWidth = 3 + pulse * 4; ctx.stroke();
        ctx.save(); ctx.globalAlpha = 1 - conv;
        text(ctx, d.label, { x: 44, y: 64, font: '500 52px "JetBrains Mono"', color: lit ? AMBER : DIM, align: 'left' });
        text(ctx, d.kind.toUpperCase(), { x: w - 44, y: 64, font: '600 30px "Space Grotesk"', color: DIM, spacing: 6, align: 'right' });
        ctx.restore();
        ctx.save(); ctx.globalAlpha = conv;
        text(ctx, '@score', { x: 44, y: 64, font: '500 52px "JetBrains Mono"', color: AMBER, align: 'left' });
        text(ctx, 'ONE PLAYHEAD', { x: w - 44, y: 64, font: '600 30px "Space Grotesk"', color: DIM, spacing: 6, align: 'right' });
        ctx.restore();
        drawCard(ctx, 110, 130, w - 220, 540, p);
        // playhead
        const y0 = h - 110;
        ctx.fillStyle = 'rgba(255,255,255,0.1)'; ctx.fillRect(60, y0, w - 120, 6);
        ctx.fillStyle = AMBER; ctx.fillRect(60, y0, (w - 120) * head, 6);
        if (lit) { ctx.beginPath(); ctx.arc(60 + (w - 120) * head, y0 + 3, 9 + pulse * 6, 0, 7); ctx.fill(); }
        if (ph) { ctx.fillStyle = 'rgba(125,211,252,0.7)'; for (let b = 0; b < 40; b++) { const bh = 6 + 22 * Math.abs(Math.sin(b * 1.7 + 0.5)) * (b / 40 < head ? 1 : 0.3); ctx.fillRect(60 + b * ((w - 120) / 40), y0 + 30 - bh / 2, 8, bh); } }
      });
    });
    const k = ev(t, 17.1, 17.9), hit = decay(t, 18.0, 0.3);
    title.at(0, -2.55).show('Every clock.', { a: k * (1 - s4out), size: 150, blur: lerp(18, 0, k), spacing: lerp(30, 2, k), glow: hit * 50 });
  }

  // ---- S5: render — the filmstrip flies past, the counter ticks
  const s5 = t >= 17.9 && t < 23.2;
  if (s5) {
    const k = E.inOutCubic(span(t, 18.0, 23.0));
    const s5in = ev(t, 18.0, 18.4), s5out = ev(t, 22.45, 22.85, E.smooth);
    const sx = lerp(38, -46, k);
    strip.mesh.position.set(sx, -0.25, -11.5); strip.mesh.rotation.set(0.1, -0.55, 0.02); strip.alpha(s5in * (1 - s5out));
    strip2.mesh.position.set(-sx * 0.55 - 10, 1.9, -15); strip2.mesh.rotation.set(0.05, 0.5, 0); strip2.alpha(0.3 * s5in * (1 - s5out));
    const fr = Math.min(1800, Math.floor(span(t, 18.2, 22.8) * 1800));
    const ms = (fr / 60);
    counter.at(-2.6, 2.55).alpha(s5in * (1 - s5out)).draw('ctr' + fr, (ctx, w, h) => {
      text(ctx, 'FRAME', { x: 0, y: 70, font: '600 30px "Space Grotesk"', color: DIM, spacing: 8, align: 'left' });
      text(ctx, String(fr).padStart(4, '0'), { x: 150, y: 70, font: '500 92px "JetBrains Mono"', color: INK, align: 'left' });
      text(ctx, '/ 1800', { x: 400, y: 70, font: '500 44px "JetBrains Mono"', color: DIM, align: 'left' });
      text(ctx, `t = ${ms.toFixed(3).padStart(6, '0')}s`, { x: 640, y: 70, font: '500 44px "JetBrains Mono"', color: CYAN, align: 'left' });
      text(ctx, 'VIRTUAL CLOCK · DETERMINISTIC', { x: 0, y: 140, font: '600 26px "Space Grotesk"', color: DIM, spacing: 6, align: 'left' });
    });
    const line1 = '$ spacetime render demos/promo/ --out film.mp4';
    const n = Math.floor(lerp(0, line1.length, span(t, 18.3, 19.4)));
    const done = ev(t, 19.7, 20.1);
    cmd.at(-2.2, -2.45).alpha(s5in * (1 - s5out)).draw('cmd' + n + done.toFixed(2), (ctx, w, h) => {
      rrect(ctx, 0, 0, w, h, 22); ctx.fillStyle = 'rgba(11,14,20,0.85)'; ctx.fill(); ctx.strokeStyle = 'rgba(255,255,255,0.1)'; ctx.lineWidth = 2; ctx.stroke();
      text(ctx, line1.slice(0, n), { x: 44, y: 80, font: '500 40px "JetBrains Mono"', color: INK, align: 'left' });
      if (n < line1.length) { ctx.fillStyle = AMBER; ctx.fillRect(44 + ctx.measureText(line1.slice(0, n)).width + 6, 58, 22, 44); }
      ctx.save(); ctx.globalAlpha = done;
      text(ctx, '✓', { x: 44, y: 170, font: '600 40px "Space Grotesk"', color: '#5eead4', align: 'left' });
      text(ctx, '1800 frames · 30.000 s · h264 · bit-identical on every run', { x: 100, y: 170, font: '500 36px "JetBrains Mono"', color: DIM, align: 'left' });
      ctx.restore();
    });
    const tk = ev(t, 20.4, 21.2), hit = decay(t, 18.0, 0.3);
    title.at(0, 0.3).show('Render it.', { a: tk * (1 - s5out), size: 230, blur: lerp(18, 0, tk), spacing: lerp(40, 4, tk), glow: hit * 40 });
  }

  // ---- S6: <script> shatters
  const s6 = t >= 22.7 && t < 27.1;
  if (s6) {
    const pop = ev(t, 22.8, 23.0, E.outBack);
    scriptPlane.at(0, 0.4).alpha(t < 23.0 ? pop : 0);
    scriptPlane.mesh.scale.setScalar(lerp(1.25, 1, pop));
    shatter.visible = t >= 23.0;
    shatter.material.uniforms.uTau.value = t - 23.0;
    shatter.position.y = 0.4; shatter.position.z = -10;
    const k = ev(t, 24.3, 25.1), out = ev(t, 26.5, 27.0, E.smooth);
    title.at(0, 0.2).show('No JavaScript.', { a: k * (1 - out), size: 210, blur: lerp(18, 0, k), spacing: lerp(40, 4, k) });
    const k2 = ev(t, 25.1, 25.9);
    sub.at(0, -1.4).alpha(k2 * (1 - out)).draw('s6sub' + k2.toFixed(2), (ctx, w, h) => {
      text(ctx, 'style  ·  motion  ·  state  ·  data  —  one declarative language', { x: w / 2, y: h / 2, font: '500 64px "Space Grotesk"', color: DIM, spacing: lerp(12, 2, k2), blur: lerp(10, 0, k2) });
    });
  }

  // ---- S7: wordmark
  if (t >= 27.0) {
    const k = ev(t, 27.0, 27.8), shim = lerp(-0.6, 1.6, span(t, 27.4, 28.9));
    mark.at(0, 0.45).show('SPACETIME', { a: k, size: 215, blur: lerp(30, 0, k), spacing: lerp(90, 30, k), shimmer: shim, glow: decay(t, 27.0, 0.35) * 40 });
    mark.mesh.scale.setScalar(lerp(1.06, 1, k));
    const rk = ev(t, 27.2, 28.1, E.outQuart);
    rule.visible = rk > 0; rule.position.set(0, -0.55, -10); rule.scale.x = 9.6 * rk; rule.material.opacity = 1;
    const k2 = ev(t, 28.0, 28.8);
    sub.at(0, -1.35).alpha(k2).draw('tag' + k2.toFixed(2), (ctx, w, h) => {
      text(ctx, 'If CSS describes space, Spacetime adds time.', { x: w / 2, y: h / 2, font: '500 70px "Space Grotesk"', color: INK, spacing: lerp(10, 1, k2), blur: lerp(10, 0, k2) });
    });
    kicker.at(0, -2.2).alpha(ev(t, 28.4, 29.0)).draw('k7', (ctx, w, h) => text(ctx, 'DECLARATIVE  ·  DETERMINISTIC  ·  ONE FILE', { x: w / 2, y: h / 2, font: '600 40px "Space Grotesk"', color: DIM, spacing: 12 }));
  }

  // ---- post
  const hit = hitEnv(t, 0.3);
  bloom.strength = 0.45 + hit * 0.8 + span(t, 21.6, 23.0) * 0.5 - ev(t, 27.0, 27.6) * 0.15;
  post.uniforms.uFlash.value = hitEnv(t, 0.07) * 0.32;
  post.uniforms.uAb.value = 0.0018 + hit * 0.006 + span(t, 21.6, 23.0) * 0.008 * (1 - ev(t, 23.0, 23.6));
  let glitch = 0;
  if (t >= 23.4 && t < 23.92) glitch = Math.floor((t - 23.4) / 0.03) % 2 === 0 ? 1 : 0.25;
  post.uniforms.uGlitch.value = glitch;
  post.uniforms.uSeed.value = frame * 0.618;
  post.uniforms.uFade.value = ev(t, 29.0, 29.95, E.smooth) + (1 - ev(t, 0.0, 0.5, E.smooth));

  composer.render();
}

// ---------------------------------------------------------------- boot
await document.fonts.load('800 100px Syne');
await document.fonts.load('500 40px "Space Grotesk"');
await document.fonts.load('600 40px "Space Grotesk"');
await document.fonts.load('500 40px "JetBrains Mono"');
await document.fonts.load('700 40px "JetBrains Mono"');
await document.fonts.ready;
// warm shaders/textures so frame 0 is not slow
for (const t of [0.5, 4, 9, 14, 20, 24, 28]) seek(t, 0);
window.__seek = seek;
window.__ready = true;
// live preview when opened by hand: scrub with the mouse, play with space
if (!navigator.webdriver && location.hash !== '#capture') {
  let t0 = performance.now() / 1000, playing = true, at = 0;
  const loop = () => { if (playing) { at = (performance.now() / 1000 - t0) % 30; } seek(at); requestAnimationFrame(loop); };
  window.addEventListener('keydown', (e) => { if (e.key === ' ') { playing = !playing; if (playing) t0 = performance.now() / 1000 - at; } });
  window.addEventListener('mousemove', (e) => { if (!playing) at = (e.clientX / innerWidth) * 30; });
  loop();
}
