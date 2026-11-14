@import "../design/index.st";

@form markup --business-title {
  <--scene name="business-title"><div class="center"><h2 class="caption">Knows your business</h2></div></--scene>
}
@form markup --business-voice {
  <--scene name="business-voice" tone="peach"><div class="business-voice__panel"><--voice kind="orange"><div class="business-voice__question"><--bubble side="out">Can you share the deck with the ad agency?</--bubble></div><div class="business-voice__answer"><--bubble>Sure. Heads up, slide 6 has sensitive<br />financials. Want me to send it without that?</--bubble></div></--voice></div></--scene>
}
@form markup --business-deck {
  <--scene name="business-deck" tone="peach"><div class="business-deck__stack"><article class="deck-slide deck-slide--previous"><small>Pendel</small><h2 class="editorial">Eight parts a cup. The battery lives in the ring.</h2><img src="assets/headphones.webp" alt="Pendel Arc's silver headphones in the preceding product specification slide" /></article><article class="deck-slide deck-slide--financial"><span class="deck-slide__number">6.</span><small>Pendel</small><div class="deck-slide__heading"><h2 class="editorial">Year one: $8M<br />in revenue.</h2><div class="deck-slide__stat"><b>$249</b><span>Price</span></div><div class="deck-slide__stat"><b>73%</b><span>Gross margin</span></div><div class="deck-slide__stat"><b>32k</b><span>Units, year one</span></div></div><p class="deck-slide__note">Break-even in month 7. Launch marketing budget, $1.5M.</p><p class="deck-slide__chart-label">Cumulative revenue, year one</p><div class="deck-slide__chart"><svg viewBox="0 0 1300 380" aria-label="Cumulative year-one revenue grows from 1.2 to 8 million dollars"><defs><linearGradient id="revenue-fill" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#f6c9a5" stop-opacity="0.7"/><stop offset="1" stop-color="#f6c9a5" stop-opacity="0"/></linearGradient></defs><path d="M 5 355 L 280 310 L 600 225 L 945 125 L 1285 10 L 1285 375 L 5 375 Z" fill="url(#revenue-fill)"/><path d="M 5 355 L 280 310 L 600 225 L 945 125 L 1285 10" fill="none" stroke="#dba87d" stroke-width="3"/></svg><span class="revenue-q1">$1.2M</span><span class="revenue-q2">$2.9M</span><span class="revenue-q3">$5.1M</span><span class="revenue-q4">$8M</span></div><footer><span>Creative brief 2026</span><span>Confidential, do not share</span></footer></article><article class="deck-slide deck-slide--next"><small>Pendel</small><img src="assets/headphones.webp" alt="Silver Pendel Arc headphones on the next slide of the launch deck" /></article></div><div class="business-deck__pointer"><--pointer kind="orange" /></div></--scene>
}
.business-voice__panel { position: absolute; left: 300px; top: 430px; }
.business-voice .voice { border-radius: 70px; }
.business-voice .bubble { font-size: 42px; }
.business-deck { background: #f9d4b9; }
.business-deck__stack { position: absolute; top: 180px; left: 250px; width: 1420px; }
.deck-slide { position: relative; width: 1420px; height: 800px; padding: 52px 70px; margin-bottom: 75px; background: white; }
.deck-slide > small { font: 35px var(--brand-editorial); }
.deck-slide h2 { font-size: 68px; line-height: 1.04; margin: 28px 0 0; }
.deck-slide--previous { height: 520px; }
.deck-slide--previous img { position: absolute; right: 200px; bottom: 20px; width: 500px; height: 300px; object-fit: cover; }
.deck-slide__number { position: absolute; left: -90px; top: 15px; color: #d87937; font-size: 50px; }
.deck-slide__heading { display: flex; gap: 70px; align-items: flex-start; }
.deck-slide__heading h2 { flex: 1; }
.deck-slide__stat { display: flex; flex-direction: column; width: 150px; gap: 12px; padding-top: 32px; }
.deck-slide__stat b { font: 42px var(--brand-editorial); }
.deck-slide__stat span { font-size: 17px; color: #888; }
.deck-slide__note { position: absolute; top: 220px; left: 700px; font-size: 18px; color: #aaa; }
.deck-slide__chart-label { font-size: 18px; color: #999; margin-top: 90px; }
.deck-slide__chart { position: absolute; left: 70px; right: 70px; top: 350px; height: 370px; }
.deck-slide__chart svg { width: 100%; height: 100%; }
.deck-slide__chart > span { position: absolute; font: 28px var(--brand-editorial); }
.revenue-q1 { left: 245px; top: 245px; }
.revenue-q2 { left: 550px; top: 165px; }
.revenue-q3 { left: 885px; top: 65px; }
.deck-slide__chart > .revenue-q4 { right: 10px; top: -50px; font-size: 44px; }
.deck-slide footer { position: absolute; left: 70px; right: 70px; bottom: 30px; display: flex; justify-content: space-between; font-size: 16px; color: #999; }
.deck-slide--next img { width: 600px; height: 500px; object-fit: cover; display: block; margin: auto; }
.business-deck__pointer { position: absolute; top: 850px; left: 1250px; }
.business-title { @on &.clip { --film-window; .caption { --film-focus; } } }
.business-voice { @on &.clip { --film-window; .business-voice__panel { scale: 0.93 -> 1 at 6% -> 1; translate-y: 0px -> -180px at 15% -> -180px; easing: --ease-out-cubic; } .voice__conversation { height: 0px -> 0px at 8% -> 140px at 16% -> 140px at 60% -> 300px at 70% -> 300px; } .business-voice__question { opacity: 0 -> 0 at 7% -> 1 at 13% -> 1; } .business-voice__answer { opacity: 0 -> 0 at 58% -> 1 at 65% -> 1; } .wave { scale-y: 0.16 -> 1 at 8% -> 0.16 at 18% -> 0.16 at 58% -> 1 at 68% -> 0.2 at 92% -> 0.2; } } }
.business-deck { @on &.clip { --film-window; .business-deck__stack { translate-y: 300px -> -590px at 44% -> -615px; blur: 6px -> 0px at 40% -> 0px; easing: --ease-in-out-cubic; } .business-deck__pointer { --film-pointer; } } }

@form markup --business-approval {
  <--scene name="business-approval" tone="peach"><div class="center"><div class="business-approval__bubble">good catch, yes</div></div></--scene>
}
.business-approval__bubble { padding: 45px 80px; border-radius: 180px; color: white; background: var(--film-orange); font-size: 145px; }
.business-approval { @on &.clip { --film-window; .business-approval__bubble { scale: 0.5 -> 1 at 25% -> 1.1; blur: 6px -> 0px at 25% -> 0px; easing: --ease-out-cubic; } } }
