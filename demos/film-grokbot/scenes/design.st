@import "../design/index.st";

@form markup --design-inspector {
  <--scene name="design-inspector"><div class="design-inspector__panel"><div class="design-inspector__top"><span class="design-inspector__avatars"><i><img src="assets/alex.webp" alt="Alex, a collaborator in the design editor" /></i><i><img src="assets/kai.webp" alt="Kai, a collaborator in the design editor" /></i><i><--bot kind="mint" /></i><--icon name="chevron-down" label="Collaborators" /></span><--icon name="play" label="Preview design" /><span class="design-inspector__share">Share</span></div><div class="design-inspector__tabs"><b>Design</b><span>Prototype</span><span>50%⌄</span></div><h3>Frame ⌄</h3><h3>Position</h3><h3>Layers</h3><div class="design-inspector__layer"># &nbsp; Ad · 1080×1350</div><div class="design-inspector__layer design-inspector__layer--active">T &nbsp; Headline</div><div class="design-inspector__layer">▧ &nbsp; Arc_render_002</div></div><div class="design-inspector__cursor"><--pointer kind="mint" /></div></--scene>
}
@form markup --design-canvas {
  <--scene name="design-canvas"><div class="design-canvas__dots"></div><div class="design-canvas__text"><h2 class="editorial design-canvas__old">Sound, reimagined.</h2><h2 class="editorial design-canvas__new">A lighter fit for longer days</h2><div class="design-canvas__selection"><i></i><i></i><i></i><i></i></div><div class="design-canvas__pointer"><--pointer kind="mint" /></div></div><div class="design-canvas__tools"><--toolstrip /></div><img class="design-canvas__product" src="assets/headphones.webp" alt="Silver Pendel Arc headphones, with the headband framing the new campaign headline" /></--scene>
}
@form markup --design-posters {
  <--scene name="design-posters"><div class="design-canvas__dots"></div><div class="design-posters__row"><div class="design-posters__item"><small>Ad · 1080×1080</small><--poster captioned="true" src="assets/square.webp" alt="A man wearing black Pendel Arc headphones in warm studio light" format="square" /></div><div class="design-posters__item"><small>Ad · 1080×1350</small><--poster captioned="true" src="assets/portrait.webp" alt="A woman wearing silver Pendel Arc headphones in a close profile portrait" /></div><div class="design-posters__item"><small>Ad · 1920×1080</small><--poster captioned="true" src="assets/wide.webp" alt="A man wearing black Pendel Arc headphones against a blue background" format="wide" /></div></div></--scene>
}
.design-inspector__panel { position: absolute; left: 0; top: -20px; width: 1920px; background: white; }
.design-inspector__top { height: 340px; display: flex; align-items: center; gap: 75px; padding: 50px 80px; }
.design-inspector__avatars { display: flex; align-items: center; flex: 1; }
.design-inspector__avatars > i { position: relative; display: grid; place-content: center; width: 190px; height: 190px; border: 10px solid white; border-radius: 50%; margin-right: -30px; background: #cae6da; font-style: normal; font-size: 85px; }
.design-inspector__avatars > i:nth-child(2) { background: #baff00; }
.design-inspector__avatars > i:nth-child(3) { background: var(--film-mint); }
.design-inspector__avatars .bot { width: 125px; height: 125px; }
.design-inspector__avatars > .icon { width: 190px; height: 190px; background: #f5f5f5; border-radius: 50%; }
.design-inspector__top > .icon { width: 150px; height: 150px; }
.design-inspector__share { background: #003e2c; color: white; padding: 60px 100px; border-radius: 50px; font-size: 90px; }
.design-inspector__tabs { display: flex; align-items: center; padding: 40px 80px 75px; gap: 45px; font-size: 90px; color: #777; border-bottom: 5px solid #e5e6e5; }
.design-inspector__tabs b { font-weight: 600; color: #161818; padding: 45px 55px; background: #f0f2f0; border-radius: 26px; }
.design-inspector__tabs span:last-child { margin-left: auto; color: black; }
.design-inspector h3 { margin: 0; padding: 80px 100px; font-size: 85px; font-weight: 600; border-bottom: 5px solid #e5e6e5; }
.design-inspector__layer { font-size: 90px; padding: 50px 100px; }
.design-inspector__layer--active { background: #d3faf1; }
.design-inspector__cursor { position: absolute; left: 970px; top: 620px; scale: 1.5; }
.design-canvas__dots { position: absolute; inset: 0; background-image: radial-gradient(#d6d7d8 2.8px, transparent 2.8px); background-size: 93px 93px; background-position: 22.5px 14px; }
.design-canvas__text { position: absolute; left: 0; top: 455px; width: 1920px; text-align: center; }
.design-canvas__text h2 { margin: 0; font-size: 134px; line-height: 1.15; font-weight: 400; white-space: nowrap; }
.design-canvas__new { position: absolute; top: 0; left: 0; width: 100%; clip-path: inset(0 100% 0 0); }
.design-canvas__selection { position: absolute; left: 440px; top: 8px; width: 1040px; height: 146px; border: 4px solid var(--film-mint); }
.design-canvas__selection i { position: absolute; width: 15px; height: 15px; background: white; border: 3px solid var(--film-mint); }
.design-canvas__selection i:nth-child(1) { left: -9px; top: -9px; }
.design-canvas__selection i:nth-child(2) { right: -9px; top: -9px; }
.design-canvas__selection i:nth-child(3) { left: -9px; bottom: -9px; }
.design-canvas__selection i:nth-child(4) { right: -9px; bottom: -9px; }
.design-canvas__pointer { position: absolute; left: 700px; top: -140px; }
.design-canvas__tools { position: absolute; bottom: 80px; left: 705px; scale: 1.2; transform-origin: bottom center; }
.design-canvas__product { position: absolute; width: 1920px; height: 1080px; inset: 0; opacity: 0; object-fit: cover; }
.design-posters__row { position: absolute; left: 196px; top: 185px; display: flex; align-items: flex-end; gap: 70px; }
.design-posters__item { flex: 0 0 auto; }
.design-posters__item > small { display: block; font-size: 28px; color: #7d8081; margin: 0 0 18px 8px; }
.design-inspector { @on &.clip { --film-window; .design-inspector__panel { translate-y: 0px -> -1230px; easing: --ease-in-out-cubic; } .design-inspector__cursor { --film-pointer; } } }
.design-canvas { @on &.clip { --film-window; .design-canvas__selection { width: 1040px -> 1040px at 40% -> 1410px at 56% -> 1410px; translate-x: 0px -> 0px at 40% -> -185px at 56% -> -185px; } .design-canvas__old { opacity: 1 -> 1 at 36% -> 0 at 40% -> 0; } .design-canvas__new { clip-path: inset(0 100% 0 0) -> inset(0 100% 0 0) at 40% -> inset(0 0% 0 0) at 56% -> inset(0 0% 0 0); } .design-canvas__pointer { --film-pointer; opacity: 1 -> 1 at 35% -> 0 at 40% -> 0; } .design-canvas__product { opacity: 0 -> 0 at 85% -> 1 at 92% -> 1; scale: 1.5 -> 1.5 at 85% -> 1 at 100%; easing: --ease-out-cubic; } } }
.design-posters { @on &.clip { --film-window; .design-posters__row { translate-x: 650px -> -190px at 65% -> -350px; scale: 1.1 -> 1 at 15% -> 1; blur: 8px -> 0px at 50% -> 0px; easing: --ease-out-cubic; } } }

.design-inspector__avatars > i { overflow: hidden; }
.design-inspector__avatars > i > img { width: 100%; height: 100%; object-fit: cover; }

.design-posters .poster--wide img { width: 475px; height: 525px; object-fit: fill; }
