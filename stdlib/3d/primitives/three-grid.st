/// Three Grid Primitive — a light-up floor of tiles (PLAN-152).
///
/// `cols × rows` tiles laid on the ground, each an emissive `Mesh` PART under one
/// `Group`, coloured from `palette` and lit by a PATTERN. A pattern is a pure
/// function of (column, row, time, phase) → brightness 0..1, kept as DATA in the
/// table below (not a switch): `checker`, `ripple`, `sweep`, `random`, `chase`.
/// Its channels:
///
///   pulse — how lit the floor is overall (0..1); drive it on the beat
///   phase — the pattern's own clock (seconds); drive it, or leave it to run
///
/// @param cols/rows  - tiles across / deep
/// @param tile       - a tile's size (world units)
/// @param gap        - space between tiles
/// @param palette    - space-separated colours the tiles cycle through
/// @param pattern    - checker | ripple | sweep | random | chase
/// @param glow       - a lit tile's emissive strength
/// @param posY       - the floor's height

%primitive three-grid(
  &el,
  name: string = "",
  cols: number = 8,
  rows: number = 8,
  tile: number = 0.9,
  gap: number = 0.06,
  palette: string = "#ff3ea5 #3ef0ff #ffe23e #7c4dff",
  pattern: string = "ripple",
  glow: number = 2.5,
  speed: number = 1,
  posY: number = 0
) {
  %emit js {
    const canvas = %&el;
    const ctx = canvas && canvas._stThree;
    if (!ctx) { console.warn('[Spacetime] three-grid: no @stage on this canvas.'); return; }
    const C = Math.max(1, Math.round(%cols)), R = Math.max(1, Math.round(%rows));
    const T = %tile, G = %gap, step = T + G;
    const colors = String(%palette).trim().split(/\s+/).map((c) => ctx.hex(c));
    // the patterns: (i, j, t) -> 0..1, with the grid's size in reach
    const hash = (i, j, k) => { const x = Math.sin(i * 127.1 + j * 311.7 + k * 74.7) * 43758.5453; return x - Math.floor(x); };
    const PATTERNS = {
      checker: (i, j, t) => ((i + j + Math.floor(t * 2)) % 2 === 0 ? 1 : 0.15),
      ripple: (i, j, t) => { const d = Math.hypot(i - (C - 1) / 2, j - (R - 1) / 2); return 0.5 + 0.5 * Math.cos(d * 1.4 - t * 5); },
      sweep: (i, j, t) => { const x = (t * 4) % (C + 4) - 2; return Math.max(0.1, 1 - Math.abs(i - x) * 0.6); },
      random: (i, j, t) => (hash(i, j, Math.floor(t * 4)) > 0.55 ? 1 : 0.08),
      chase: (i, j, t) => (((i + j * C) - Math.floor(t * 12)) % 7 === 0 ? 1 : 0.1),
    };
    const pattern = PATTERNS[%pattern] || PATTERNS.ripple;
    const name = String(%name || '').replace(/^&/, '');
    const speed = %speed, glow = %glow;
    const group = ctx.part('Group', { position: [0, %posY, 0], rotation: [0, 0, 0], scale: 1 }, {
      name,
      update: (g, t) => {
        const tt = g.phase ? g.phase : t * speed;
        const p = g.channels && 'pulse' in g.channels ? g.pulse : 1;
        for (const tl of g.kids) tl.mat.emissiveIntensity = glow * p * pattern(tl.i, tl.j, tt);
      },
    });
    for (let j = 0; j < R; j++) for (let i = 0; i < C; i++) {
      const c = colors[(i + j * 3) % colors.length];
      const tl = ctx.part('Mesh', { position: [(i - (C - 1) / 2) * step, 0, (j - (R - 1) / 2) * step], rotation: [0, 0, 0], scale: 1, castShadow: false },
        { parent: group, shape: `box(${T}, 0.06, ${T})`, geom: { type: 'box', args: [T, 0.06, T] },
          material: { color: '#111216', roughness: 0.35, metalness: 0.1, emissive: c, emissiveIntensity: 0 } });
      tl.i = i; tl.j = j;
    }
    %yield true -> $added;
  }

  %exports {
    $added: bool
  }
}
