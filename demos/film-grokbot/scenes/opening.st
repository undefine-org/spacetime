@import "../design/index.st";

@form markup --opening {
  <--scene name="opening"><div class="opening__world"><div class="opening__character"><--bot kind="purple" label="Launch Bot" /></div></div><div class="opening__notification"><--bot kind="purple" label="Launch Bot notification" /><div><div class="opening__notification-head">Launch Bot <time>7:00 AM</time></div><p>New headphones launching in 2 weeks.<br />I created a channel to keep us on track.</p></div></div></--scene>
}
@form markup --introduction {
  <--scene name="introduction"><div class="center"><div class="introduction__line"><span>Introducing</span><span class="introduction__product"> Grok Bot</span><span class="introduction__beta">beta</span><span class="introduction__version">1.0</span></div></div><div class="introduction__cast"><span class="cast-black"><--bot kind="black" /></span><span class="cast-mint"><--bot kind="mint" /></span><span class="cast-blue"><--bot /></span><span class="cast-red"><--bot kind="red" /></span><span class="cast-orange"><--bot kind="orange" /></span></div></--scene>
}
.scene.opening { background: linear-gradient(#a590e9, #e4e1f2); }
.opening__world { position: absolute; inset: 0; background: linear-gradient(#130b30, #39216a); }
.opening__character { position: absolute; top: 215px; left: 52px; rotate: -24deg; }
.opening__character .bot { width: 1800px; height: 1800px; }
.opening__character .bot__eyes { gap: 16%; }
.opening__notification { position: absolute; left: 245px; right: 245px; top: 394px; padding: 44px 48px; display: flex; gap: 38px; border-radius: 78px; color: white; background: #81739c66; font-size: 52px; opacity: 0; }
.opening__notification > .bot { width: 110px; height: 110px; align-self: center; }
.opening__notification > div { flex: 1; }
.opening__notification-head { display: flex; justify-content: space-between; line-height: 1.4; }
.opening__notification time { opacity: 0.55; }
.opening__notification p { margin: 0; line-height: 1.25; }
.introduction__line { display: flex; align-items: center; white-space: pre; font-size: 92px; letter-spacing: -1.7px; }
.introduction__beta { font-size: 57px; letter-spacing: -1px; padding: 12px 30px; margin-left: 45px; border-radius: 80px; background: #f6f6f5; }
.introduction__version { position: absolute; left: 855px; font-size: 64px; line-height: 1; color: white; background: black; border-radius: 45px; padding: 12px 30px; letter-spacing: -1px; opacity: 0; }
.introduction__cast { position: absolute; left: 1140px; top: 355px; width: 440px; height: 450px; }
.introduction__cast > span { position: absolute; }
.introduction__cast .bot { width: 110px; height: 110px; }
.cast-black { left: 20px; top: 30px; }
.cast-black .bot { width: 75px; height: 75px; }
.cast-mint { left: 155px; top: 0; }
.cast-mint .bot { width: 100px; height: 100px; }
.cast-blue { left: 335px; top: -5px; }
.cast-blue .bot { width: 120px; height: 120px; }
.cast-red { left: 115px; top: 260px; }
.cast-red .bot { width: 60px; height: 60px; }
.cast-orange { left: 288px; top: 310px; }
.cast-orange .bot { width: 60px; height: 60px; }
.opening { @on &.clip { --film-window; .opening__character { translate-y: 170px -> -230px at 38% -> -230px; rotate: 8deg -> -16deg at 38% -> -16deg; } .opening__world { opacity: 1 -> 1 at 35% -> 0 at 49% -> 0; blur: 0px -> 0px at 35% -> 30px at 49% -> 30px; } .opening__notification { opacity: 0 -> 0 at 37% -> 1 at 49% -> 1; translate-y: 150px -> 150px at 37% -> 0px at 65% -> 0px; blur: 10px -> 10px at 37% -> 0px at 49% -> 0px; } } }
.introduction { @on &.clip { --film-window; .introduction__line { scale: 0.94 -> 1 at 18% -> 1; easing: --ease-out-cubic; } .introduction__product { opacity: 0 -> 0 at 16% -> 1 at 30% -> 1; } .introduction__beta { opacity: 0 -> 0 at 28% -> 1 at 40% -> 1 at 40% -> 0 at 48% -> 0; } .introduction__version { opacity: 0 -> 0 at 42% -> 1 at 52% -> 1; scale: 0.1 -> 0.1 at 42% -> 1 at 52% -> 1; } .introduction__cast { opacity: 0 -> 0 at 40% -> 1 at 52% -> 1; scale: 0.1 -> 0.1 at 40% -> 1 at 52% -> 1; rotate: -22deg -> -22deg at 40% -> 0deg at 52% -> 0deg; } } }

.introduction__line { translate: 6px 0px; }
.introduction__line > span:first-child, .introduction__product { position: relative; top: -7px; }
.opening { @on &.clip { .opening__notification .bot { scale: 3.5 -> 3.5 at 45% -> 1 at 70% -> 1; } .opening__notification .bot__eyes { opacity: 0 -> 0 at 45% -> 1 at 70% -> 1; } } }
