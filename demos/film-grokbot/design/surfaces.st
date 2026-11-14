// Official xAI vector geometry; shape and eyes stay separate, color and
// pose remain editable. Geometry is data, shared by every component size.
.bot { position: relative; display: inline-block; width: 100px; height: 100px; flex: 0 0 auto; color: var(--film-blue); vertical-align: middle; --bot-kind: blue; }
.bot__body { position: absolute; inset: 0; background: currentColor; mask: url("assets/bots/blue-body.svg") center / contain no-repeat; }
.bot__eyes { position: absolute; inset: 0; background: white; mask: url("assets/bots/blue-eyes.svg") center / contain no-repeat; }
.bot__eyes i { display: none; }
.bot--black { color: #000; }
.bot--black .bot__body { mask-image: url("assets/bots/black-body.svg"); }
.bot--black .bot__eyes { mask-image: url("assets/bots/black-eyes.svg"); }
.bot--orange { color: var(--film-orange); }
.bot--orange .bot__body { mask-image: url("assets/bots/orange-body.svg"); }
.bot--orange .bot__eyes { mask-image: url("assets/bots/orange-eyes.svg"); }
.bot--mint { color: var(--film-mint); }
.bot--mint .bot__body { mask-image: url("assets/bots/mint-body.svg"); }
.bot--mint .bot__eyes { mask-image: url("assets/bots/mint-eyes.svg"); }
.bot--red { color: var(--film-red); }
.bot--red .bot__body { mask-image: url("assets/bots/red-body.svg"); }
.bot--red .bot__eyes { mask-image: url("assets/bots/red-eyes.svg"); }
.bot--purple { color: var(--film-purple); }
.bot--purple .bot__body { mask-image: url("assets/bots/purple-body.svg"); }
.bot--purple .bot__eyes { mask-image: url("assets/bots/purple-eyes.svg"); }
.bot--purple .bot__body { background: radial-gradient(circle at 23% 15%, #a17bff, #733ee0 53%, #41207b); }
.icon { display: inline-flex; width: 56px; height: 56px; justify-content: center; align-items: center; flex: 0 0 auto; }
.icon img { display: block; width: 57%; height: 57%; }
.voice { width: 1320px; padding: 36px 46px; border-radius: 86px; background: white; box-shadow: 0 26px 36px #23303d12, 0 3px 10px #23303d09; color: var(--film-blue); }
.voice--orange { color: var(--film-orange); }
.voice__bar { display: flex; align-items: center; gap: 20px; height: 120px; }
.voice__bar > .bot { width: 100px; height: 100px; }
.voice__bar > .icon { width: 120px; height: 120px; border-radius: 50%; background: var(--film-ui); }
.voice__bar > .icon img { width: 48px; height: 48px; }
.voice__spacer { flex: 1; }
.voice__end { display: flex; width: 128px; height: 128px; border-radius: 50%; background: #ea1942; align-items: center; justify-content: center; }
.voice__end .icon { width: 100px; height: 100px; filter: invert(1); }
.voice__conversation { overflow: hidden; }
.voice__conversation:empty { display: none; }
.wave { display: inline-flex; align-items: center; gap: 10px; height: 90px; margin-left: 24px; }
.wave i { display: block; width: 8px; height: 8px; border-radius: 8px; background: currentColor; }
.wave i:nth-child(6), .wave i:nth-child(10) { height: 20px; }
.wave i:nth-child(7), .wave i:nth-child(9) { height: 34px; }
.wave i:nth-child(8) { height: 54px; }
.bubble-row { display: flex; padding-top: 34px; }
.bubble-row--out { justify-content: flex-end; }
.bubble { display: inline-block; padding: 22px 34px; border-radius: 62px; background: var(--film-bubble); font-size: 44px; line-height: 1.35; color: var(--film-ink); }
.bubble--out { color: white; background: var(--film-blue); }
.voice--orange .bubble--out { background: var(--film-orange); }
.bubble--in { background: #f5f7f4; }
.message { display: flex; align-items: flex-start; gap: 24px; font-size: 44px; }
.message__avatar > .bot { width: 88px; height: 88px; }
.message__avatar { display: grid; place-content: center; width: 102px; height: 102px; border-radius: 50%; background: var(--film-blue); }
.message__avatar .bot__body { color: white; }
.message__avatar .bot__eyes { background: var(--film-blue); }
.message__header { display: flex; align-items: center; gap: 18px; font-size: 35px; white-space: nowrap; margin-bottom: 20px; }
.message__header b { font-weight: 600; }
.message__header time { font-size: 34px; }
.message__app { font-size: 25px; color: #777; background: #f5f5f5; }
.message__app:empty { display: none; }
.message__content { line-height: 1.45; }
.message__content p { margin: 0; }
.mention { color: #176083; }
.reaction { display: inline-flex; align-items: center; background: #f0f2f5; border-radius: 40px; padding: 6px 22px; font-size: 37px; line-height: 1.4; margin-top: 16px; }
.composer { position: absolute; bottom: 45px; left: 30px; right: 30px; display: flex; align-items: center; gap: 52px; color: #bcc3c7; opacity: 0.38; font-size: 47px; }
.composer > span:nth-child(2) { flex: 1; }
.composer .icon { width: 90px; height: 110px; }
.composer .icon:first-child { background: #eaf0f8; border-radius: 38px; }
.pointer { position: absolute; display: block; width: 150px; height: 180px; color: var(--film-blue); filter: drop-shadow(0 3px 2px #0002); }
.pointer--mint { color: var(--film-mint); }
.pointer--orange { color: var(--film-orange); }
.pointer__arrow { position: absolute; width: 80px; height: 95px; background: white; clip-path: polygon(5% 0,100% 65%,56% 70%,25% 100%); rotate: -14deg; }
.pointer__arrow::after { content: ""; position: absolute; inset: 10px; background: currentColor; clip-path: polygon(0 0,100% 65%,56% 70%,25% 100%); }
.pointer__avatar { position: absolute; top: 86px; left: 52px; width: 100px; height: 100px; background: white; border-radius: 50%; padding: 10px; }
.pointer__avatar .bot { width: 80px; height: 80px; }
.participant { position: relative; width: 766px; height: 398px; overflow: hidden; border-radius: 30px; background: var(--film-cloud); }
.participant > .bot { position: absolute; width: 240px; height: 240px; left: 263px; top: 80px; }
.participant__name { position: absolute; bottom: 23px; left: 32px; color: white; font-size: 28px; }
.participant--bot .participant__name { color: #777d83; }
.filmstrip { position: absolute; inset: 0; overflow: hidden; --frame: 0; container-type: size; }
.filmstrip img {
  --cell-width: max(100cqw, calc(100cqh * var(--aspect)));
  --cell-height: calc(var(--cell-width) / var(--aspect));
  display: block; position: absolute; max-width: none;
  width: calc(var(--cell-width) * var(--columns)); height: calc(var(--cell-height) * var(--rows));
  left: calc((100cqw - var(--cell-width)) / 2 - var(--cell-width) * mod(round(down, var(--frame)), var(--columns)));
  top: calc((100cqh - var(--cell-height)) / 2 - var(--cell-height) * round(down, var(--frame) / var(--columns)));
}

.toolstrip { display: flex; align-items: center; gap: 10px; height: 82px; padding: 10px 20px; border-radius: 18px; background: white; box-shadow: 0 8px 20px #0001; }
.toolstrip .icon { width: 60px; height: 60px; }
.toolstrip__active { display: flex; border-radius: 10px; background: var(--film-mint); }
.toolstrip__active .icon { filter: invert(1); }
.poster { position: relative; overflow: hidden; width: 500px; height: 625px; border-radius: 12px; background: #292126; color: white; flex: 0 0 auto; }
.poster img { width: 100%; height: 100%; object-fit: cover; }
.poster--square { width: 585px; height: 590px; }
.poster--wide { width: 800px; height: 450px; }
.poster__brand { position: absolute; top: 18px; left: 0; width: 100%; text-align: center; font: 48px var(--brand-editorial); letter-spacing: -2px; }
.poster__line { position: absolute; bottom: 18px; left: 0; width: 100%; text-align: center; font: 38px var(--brand-editorial); letter-spacing: -1.5px; }

.bot--red .bot__body { rotate: -22deg; }
