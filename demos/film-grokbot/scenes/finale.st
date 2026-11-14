@import "../design/index.st";

@form markup --launch-message {
  <--scene name="launch-message"><div class="launch-message__thread"><--message name="Launch Bot" kind="purple" time="10:06 AM" app="APP"><p>The new headphones campaign is live! 🚀</p></--message></div></--scene>
}
@form markup --campaign {
  <--scene name="campaign"><div class="campaign__mosaic"><--poster src="assets/square.webp" alt="The launched Pendel Arc campaign's warm portrait of a man in black headphones" format="square" /><--poster src="assets/portrait.webp" alt="The launched campaign's silver headphone portrait" /><--poster src="assets/wide.webp" alt="The launched campaign's blue-toned headphone portrait" format="wide" /><--poster src="assets/portrait.webp" alt="Another scale of the silver headphone campaign portrait" /></div><img class="campaign__hero" src="assets/campaign-orange.webp" alt="The Pendel Arc campaign portrait, black headphones against a vivid orange background" /><div class="campaign__reaction"><--reaction>🚀 22</--reaction> <--reaction>♧ 19</--reaction> <span class="campaign__heart"><--bot kind="red" label="Grok Bot heart reaction" /> 47</span> <--reaction>💯 19</--reaction> <--reaction>🛒 20</--reaction><div class="campaign__tooltip"><--bot kind="red" label="Grok Bot heart reaction" /><p>You, Launch Bot, Research Bot,<br />Copy Bot, Deck Bot and 44 others<br /><span>reacted with :grok-bot-heart:</span></p></div></div></--scene>
}
@form markup --heart {
  <--scene name="heart" tone="pink"><div class="center"><div class="heart__character"><--bot kind="red" label="Red Grok Bot heart" /></div></div></--scene>
}
@form markup --goodbye {
  <--scene name="goodbye"><div class="center"><p class="caption"><span class="goodbye__prefix">Say hi to</span><span class="goodbye__new"> your new</span><span class="goodbye__teammates"> teammates</span></p></div></--scene>
}
@form markup --cast {
  <--scene name="cast"><div class="cast__row"><--bot /><--bot kind="orange" /><--bot kind="black" /><--bot kind="mint" /><--bot kind="red" /></div></--scene>
}
@form markup --wordmark {
  <--scene name="wordmark"><div class="center"><div class="wordmark__lockup"><--bot kind="black" label="Grok Bot" /><span>Grok Bot</span></div></div></--scene>
}
.launch-message__thread { position: absolute; left: 320px; top: 420px; }
.launch-message .message { font-size: 53px; }
.launch-message .message__avatar { border-radius: 25px; background: var(--film-purple); }
.launch-message .message__avatar .bot__body { background: linear-gradient(135deg, #aa88ff, #6132b9); }
.launch-message .message__avatar .bot__eyes { background: white; }
.campaign__mosaic { position: absolute; left: -100px; top: -100px; width: 2200px; height: 1500px; display: grid; grid-template-columns: 800px 800px; grid-template-rows: 650px 650px; gap: 55px; rotate: -3deg; }
.campaign__mosaic .poster { width: 100%; height: 100%; }
.campaign__reaction { position: absolute; left: 550px; top: 940px; display: flex; gap: 12px; align-items: center; white-space: nowrap; background: white; padding: 25px; border-radius: 30px; }
.campaign__reaction .reaction { font-size: 35px; margin: 0; }
.campaign__heart { display: inline-flex; align-items: center; border: 3px solid #80bee5; border-radius: 40px; padding: 2px 14px; font-size: 37px; background: #eff9ff; }
.campaign__heart .bot { width: 55px; height: 55px; }
.campaign__tooltip { position: absolute; left: 60px; bottom: 120px; width: 900px; padding: 45px; background: white; border-radius: 60px; box-shadow: 0 8px 30px #0002; text-align: center; }
.campaign__tooltip::after { content: ""; position: absolute; left: 420px; bottom: -36px; border-left: 40px solid transparent; border-right: 40px solid transparent; border-top: 40px solid white; }
.campaign__tooltip > .bot { width: 130px; height: 130px; }
.campaign__tooltip p { font-size: 43px; line-height: 1.2; }
.campaign__tooltip p span { color: #666; }
.heart__character .bot { width: 430px; height: 430px; }
.heart__character { transform-style: preserve-3d; }
.goodbye .caption { white-space: nowrap; font-size: 88px; }
.goodbye__new, .goodbye__teammates { display: inline-block; overflow: hidden; vertical-align: bottom; white-space: pre; }
.cast__row { position: absolute; top: 450px; left: 530px; display: flex; gap: 35px; align-items: center; }
.cast__row .bot { width: 140px; height: 140px; }
.wordmark__lockup { display: flex; align-items: center; gap: 32px; font-size: 111px; letter-spacing: -4px; }
.wordmark__lockup .bot { width: 115px; height: 115px; }
.launch-message { @on &.clip { --film-window; .launch-message__thread { --film-arrive; } } }
.campaign { @on &.clip { --film-window; .campaign__mosaic { translate-y: 200px -> -130px; scale: 1.2 -> 1; easing: --ease-out-cubic; } .campaign__reaction { opacity: 0 -> 0 at 50% -> 1 at 57% -> 1; translate-y: 90px -> 90px at 50% -> 0px at 57% -> 0px; } .campaign__tooltip { opacity: 0 -> 0 at 68% -> 1 at 73% -> 1; scale: 0.8 -> 0.8 at 68% -> 1 at 73% -> 1; } } }
.heart { @on &.clip { --film-window; .heart__character { rotate-y: -75deg -> 0deg at 35% -> 35deg at 70% -> 10deg; scale: 0.65 -> 1 at 35% -> 1; blur: 8px -> 0px at 35% -> 0px; easing: --ease-out-cubic; } } }
.goodbye { @on &.clip { --film-window; .caption { opacity: 0 -> 1 at 10% -> 1; } .goodbye__new { width: 0px -> 0px at 25% -> 360px at 40% -> 360px; } .goodbye__teammates { width: 0px -> 0px at 50% -> 440px at 65% -> 440px; } } }
.cast { @on &.clip { --film-window; .cast__row { scale: 0.85 -> 1 at 15% -> 1 at 70% -> 0.2; blur: 8px -> 0px at 15% -> 0px at 70% -> 8px; easing: --ease-out-cubic; } } }
.wordmark { @on &.clip { --film-window; .wordmark__lockup .bot { scale: 2 -> 1 at 30% -> 1; translate-x: 260px -> 0px at 40% -> 0px; rotate: -35deg -> 0deg at 40% -> 0deg; easing: --ease-in-out-cubic; } .wordmark__lockup > span { opacity: 0 -> 0 at 32% -> 1 at 45% -> 1; blur: 6px -> 6px at 32% -> 0px at 45% -> 0px; translate-x: -30px -> 0px at 45% -> 0px; } } }

.campaign__hero { position: absolute; left: 330px; top: -80px; width: 1300px; height: 1020px; object-fit: cover; opacity: 0; }
.campaign { @on &.clip { .campaign__hero { opacity: 0 -> 0 at 54% -> 1 at 55% -> 1; } .scene__camera { scale: 1 -> 1 at 82% -> 4.8 at 99%; translate-y: 0px -> 0px at 82% -> -700px at 99%; easing: --ease-in-out-cubic; } } }
