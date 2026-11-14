@import "../design/index.st";

@form markup --collaboration-title {
  <--scene name="collaboration-title"><div class="center"><h2 class="caption">Works with other bots</h2></div></--scene>
}
@form markup --collaboration {
  <--scene name="collaboration"><div class="collaboration__thread"><div class="collaboration__research"><--message name="Research Bot" time="11:02 AM" app="APP"><p><span class="mention">@marketing</span> Can we work this feedback<br />into the ad concepts? cc <span class="mention">@Copy Bot</span></p></--message></div><div class="collaboration__copy"><--message name="Copy Bot" kind="mint" time="11:02 AM" app="APP"><p>I'm on it.</p><--reaction>👍 1</--reaction></--message></div></div></--scene>
}
@form markup --collaborators {
  <--scene name="collaborators"><div class="collaborators__row"><img src="assets/alex.webp" alt="Alex's collaborator avatar" /><span class="collaborators__human"><img src="assets/kai.webp" alt="Kai, a collaborator wearing a blue shirt" /></span><span class="collaborators__bot"><--bot kind="mint" label="Copy Bot collaborator" /></span><span class="collaborators__more"><--icon name="chevron-down" label="Show collaborators" /></span></div></--scene>
}
.collaboration__thread { position: absolute; left: 210px; top: 420px; width: 1500px; }
.collaboration .message { font-size: 54px; gap: 35px; }
.collaboration .message__avatar { border-radius: 24px; background: linear-gradient(130deg, #93ddff, #1273e1); width: 125px; height: 125px; }
.collaboration .message__avatar .bot { width: 100px; height: 100px; }
.collaboration .message__avatar .bot__body { color: #dcefff; }
.collaboration .message__avatar .bot__eyes { background: white; }
.collaboration .message__header { font-size: 48px; margin-bottom: 18px; }
.collaboration .message__header time { color: #999; font-size: 39px; }
.collaboration__copy { margin-top: 100px; }
.collaboration__copy .message__avatar { background: linear-gradient(130deg, #9bffe4, #00b28c); }
.collaborators__row { position: absolute; display: flex; align-items: center; left: 585px; top: 390px; }
.collaborators__row > * { width: 260px; height: 260px; border-radius: 50%; border: 12px solid white; margin-left: -45px; position: relative; }
.collaborators__row > img { object-fit: cover; object-position: left top; }
.collaborators__human { display: grid; place-content: center; background: #baff00; color: #252525; font-size: 160px; font-family: var(--brand-editorial); }
.collaborators__bot { display: grid; place-content: center; background: var(--film-mint); }
.collaborators__bot .bot { width: 190px; height: 190px; }
.collaborators__bot .bot__body { color: white; }
.collaborators__bot .bot__eyes { background: var(--film-mint); }
.collaborators__more { display: grid; place-content: center; background: #f5f6f5; }
.collaborators__more .icon { width: 170px; height: 170px; }
.collaboration-title { @on &.clip { --film-window; .caption { --film-focus; } } }
.collaboration { @on &.clip { --film-window; .collaboration__thread { scale: 0.72 -> 1 at 17% -> 1; translate-y: 70px -> 0px at 17% -> -140px at 55% -> -140px; easing: --ease-out-cubic; } .collaboration__research { opacity: 0 -> 1 at 8% -> 1; } .collaboration__copy { opacity: 0 -> 0 at 44% -> 1 at 51% -> 1; translate-y: 45px -> 45px at 44% -> 0px at 51% -> 0px; blur: 6px -> 6px at 44% -> 0px at 51% -> 0px; } .reaction { opacity: 0 -> 0 at 74% -> 1 at 80% -> 1; } } }
.collaborators { @on &.clip { --film-window; .collaborators__row { scale: 0.9 -> 1 at 25% -> 1; easing: --ease-out-cubic; } .collaborators__bot { translate-y: -80px -> 0px at 45% -> 0px; blur: 7px -> 0px at 45% -> 0px; } } }

.collaborators__human img { width: 100%; height: 100%; border-radius: 50%; object-fit: cover; }
