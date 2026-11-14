@import "../design/index.st";

@form markup --voice-invitation {
  <--scene name="voice-invitation" tone="blue"><div class="invitation__panel"><--voice><div class="invitation__ask"><--bubble side="out">Hey, can you join the call at 10?</--bubble></div><div class="invitation__reply"><--bubble>Of course.</--bubble></div></--voice></div><div class="invitation__type">Hey, can you join the call at 10?</div></--scene>
}
@form markup --admit {
  <--scene name="admit"><div class="admit__panel"><p>Product standup</p><div class="admit__buttons"><span>View all</span><span class="admit__button">Admit</span></div></div><div class="admit__hand"><--icon name="hand" label="Pointer admitting the bot to the meeting" /></div></--scene>
}
@form markup --meeting {
  <--scene name="meeting"><div class="meeting__grid"><--participant name="Alex"><--filmstrip src="assets/alex-atlas.webp" alt="Alex, wearing a yellow shirt, speaking from her room during the product meeting" /></--participant><--participant name="Maya"><--filmstrip src="assets/maya-atlas.webp" alt="Maya, wearing a mauve shirt, listening from her living room during the product meeting" /></--participant><--participant name="Kai"><--filmstrip src="assets/kai-atlas.webp" alt="Kai, wearing a blue shirt, participating from a cafe in the product meeting" /></--participant><div class="participant participant--bot"><--bot /><span class="participant__name">Research Bot</span></div></div><div class="meeting__controls"><--icon name="ellipsis" label="More meeting controls" /><--icon name="mic" label="Meeting microphone" /><--icon name="chevron-down" label="Audio options" /><--icon name="video" label="Meeting camera" /><--icon name="monitor-up" label="Share screen" /><--icon name="smile" label="Meeting reactions" /><--icon name="captions" label="Captions" /><--icon name="hand" label="Raise hand" /><--icon name="ellipsis" label="More" /><span class="meeting__leave"><--icon name="x" label="Leave meeting" /></span></div></--scene>
}
.invitation__panel { position: absolute; left: 170px; top: 430px; }
.invitation__panel .voice { width: 1580px; border-radius: 70px; }
.invitation__type { position: absolute; left: -650px; top: 330px; color: var(--film-blue); font-size: 405px; letter-spacing: -18px; white-space: nowrap; opacity: 0; }
.admit__panel { position: absolute; top: -150px; left: -700px; width: 2200px; height: 1100px; border-radius: 90px; background: white; box-shadow: 40px 70px 90px #2222; padding: 100px; }
.admit__panel p { font-size: 100px; }
.admit__buttons { position: absolute; bottom: 130px; right: 130px; display: flex; gap: 160px; align-items: center; color: #176bc8; font-size: 125px; font-weight: 600; }
.admit__button { padding: 40px 110px; background: #edf3ff; border-radius: 150px; }
.admit__hand { position: absolute; left: 1025px; top: 660px; rotate: -12deg; }
.admit__hand .icon { width: 280px; height: 280px; }
.admit__hand .icon img { width: 100%; height: 100%; }
.meeting__grid { position: absolute; top: 98px; left: 178px; display: grid; grid-template-columns: 766px 766px; gap: 24px 32px; }
.meeting__controls { position: absolute; display: flex; align-items: center; gap: 15px; left: 600px; bottom: 54px; height: 80px; padding: 10px 22px; border: 1px solid #ddd; border-radius: 45px; box-shadow: 0 3px 10px #0001; }
.meeting__controls .icon { width: 52px; height: 52px; border-radius: 50%; background: #f5f5f5; }
.meeting__leave { background: #df2425; display: flex; border-radius: 40px; }
.meeting__leave .icon { background: transparent; filter: invert(1); width: 80px; }
.voice-invitation { @on &.clip { --film-window; .invitation__panel { scale: 0.8 -> 0.8 at 20% -> 1 at 55% -> 1; translate-y: 0px -> 0px at 35% -> -170px at 50% -> -170px; easing: --ease-out-cubic; } .invitation__type { opacity: 0 -> 0 at 17% -> 1 at 20% -> 1 at 35% -> 0 at 38% -> 0; translate-x: 900px -> -800px at 38% -> -800px; } .voice__conversation { height: 0px -> 0px at 30% -> 160px at 45% -> 160px at 58% -> 300px at 70% -> 300px; } .invitation__ask { opacity: 0 -> 0 at 38% -> 1 at 46% -> 1; } .invitation__reply { opacity: 0 -> 0 at 58% -> 1 at 67% -> 1; } .wave { scale-y: 0.15 -> 1 at 8% -> 0.15 at 19% -> 0.15; } } }
.admit { @on &.clip { --film-window; .scene__camera { scale: 0.92 -> 1 at 30% -> 1; easing: --ease-out-cubic; } .admit__hand { translate-y: 90px -> 0px at 45% -> 0px; scale: 1 -> 1 at 50% -> 0.9 at 60% -> 1 at 70% -> 1; } } }
.meeting { @on &.clip { --film-window; .meeting__grid { scale: 1.04 -> 1 at 14% -> 1; blur: 5px -> 0px at 14% -> 0px; easing: --ease-out-cubic; } .filmstrip { --frame: 0 -> 0 at 22.94% -> 119 at 93.53% -> 119; } .participant--bot .bot__eyes { scale-y: 1 -> 1 at 20% -> 0.45 at 30% -> 1 at 40% -> 1 at 70% -> 0.4 at 80% -> 1; } } }

// Admit the fourth participant: three portrait tiles become a 2×2 call.
.meeting { @on &.clip {
  .participant { width: 500px -> 500px at 12% -> 766px at 24% -> 766px; height: 800px -> 800px at 12% -> 398px at 24% -> 398px; }
  .participant:nth-child(2) { translate-x: 250px -> 250px at 12% -> 0px at 24% -> 0px; }
  .participant:nth-child(3) { translate-x: 525px -> 525px at 12% -> 0px at 24% -> 0px; translate-y: -824px -> -824px at 12% -> 0px at 24% -> 0px; }
  .participant--bot { opacity: 0 -> 0 at 12% -> 1 at 24% -> 1; }
} }
