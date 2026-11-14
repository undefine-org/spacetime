@import "../design/index.st";

@form markup --research-title {
  <--scene name="research-title"><div class="center"><h2 class="caption">Learns and makes recommendations</h2></div></--scene>
}
@form markup --research {
  <--scene name="research"><div class="research__chat"><div class="research__lookup"><--message><div class="research__bubble">Let me look up the latest tester feedback.</div></--message></div><div class="research__result"><--message><div class="research__bubble">Testers love how light the headphones are.</div><--reaction>♥ 2</--reaction> <--reaction>♧ 1</--reaction></--message></div><div class="research__offer"><--message><div class="research__bubble">Want me to ping marketing?</div><div class="research__approved"><--reaction>👍 1</--reaction></div></--message></div><--composer /></div></--scene>
}
@form markup --feedback-chart {
  <--scene name="feedback-chart"><div class="feedback"><h3>What testers mentioned</h3><div class="feedback__legend"><span>━ <b>Lightness</b></span><span>━ <b>Comfort</b></span><span>━ <b>Price</b></span></div><div class="feedback__plot"><div class="feedback__grid"></div><svg class="feedback__curves" viewBox="0 0 1700 700" aria-label="Tester feedback: lightness rises to 89 percent while comfort and price decline"><defs><linearGradient id="feedback-fill" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#168bfa" stop-opacity="0.2"/><stop offset="1" stop-color="#168bfa" stop-opacity="0"/></linearGradient></defs><path d="M 0 595 C 430 270 850 145 1700 155 L 1700 700 L 0 700 Z" fill="url(#feedback-fill)"/><path class="feedback__lightness" fill="none" stroke="#168bfa" stroke-width="10"/><path d="M 0 490 C 700 510 1100 640 1700 640" fill="none" stroke="#aab1b4" stroke-width="8"/><path d="M 0 570 C 650 600 1100 660 1700 680" fill="none" stroke="#d7dcde" stroke-width="7"/></svg><div class="feedback__axis"><span>20</span><span>40</span><span>60</span><span>80</span></div><div class="feedback__marker"><div class="feedback__tooltip"><p>80 testers</p><div><span>● Lightness</span><b>89%</b></div><div><span>● Comfort</span><b>33%</b></div><div><span>● Price</span><b>30%</b></div></div><span class="feedback__dot"></span><--pointer /></div></div></div></--scene>
}
.research__chat { position: absolute; inset: 0 50px; border-left: 2px solid #f1f2f3; border-right: 2px solid #f1f2f3; }
.research__lookup { position: absolute; left: 185px; top: 350px; }
.research__bubble { background: var(--film-bubble); border-radius: 65px; padding: 27px 42px; white-space: nowrap; font-size: 56px; }
.research__result { position: absolute; left: 185px; top: 380px; }
.research__offer { position: absolute; left: 185px; top: 330px; }
.research__approved { position: absolute; right: 80px; top: -36px; }
.research__approved .reaction { background: #dbe9ff; border: 2px solid #5a98d7; }
.feedback { position: absolute; inset: 0; padding: 20px 80px; background: #f8fafc; }
.feedback h3 { font-size: 90px; font-weight: 400; letter-spacing: -3px; margin: 0 0 35px; }
.feedback__legend { display: flex; gap: 70px; font-size: 65px; color: #aaaeb2; }
.feedback__legend span:first-child { color: var(--film-blue); }
.feedback__legend b { color: #6c7175; font-weight: 400; }
.feedback__plot { position: absolute; left: 160px; right: 30px; top: 350px; height: 700px; }
.feedback__grid { position: absolute; inset: 0; background: repeating-linear-gradient(to bottom, #d8dcdf 0 3px, transparent 3px 175px); }
.feedback__curves { position: absolute; width: 100%; height: 100%; overflow: visible; }
.feedback__axis { position: absolute; left: 0; right: 0; bottom: -35px; display: flex; justify-content: space-between; font-size: 50px; color: #8e9295; }
.feedback__marker { position: absolute; left: 0; top: 0; offset-path: var(--feedback-curve); offset-rotate: 0deg; }
.feedback__dot { position: absolute; left: -15px; top: -15px; width: 35px; height: 35px; border-radius: 50%; border: 9px solid var(--film-blue); background: white; }
.feedback__marker .pointer { left: 20px; top: 20px; }
.feedback__tooltip { position: absolute; width: 530px; left: -230px; bottom: 85px; border: 2px solid #e5e7e9; border-radius: 55px; background: #ffffffeb; padding: 44px; font-size: 44px; box-shadow: 0 3px 6px #0001; }
.feedback__tooltip p { margin: 0 0 25px; color: #666; }
.feedback__tooltip div { display: flex; justify-content: space-between; gap: 20px; line-height: 1.9; }
.feedback__tooltip b { font-weight: 400; }
.research-title { @on &.clip { --film-window; .caption { --film-focus; } } }
.research { @on &.clip { --film-window;
  .research__lookup { opacity: 0 -> 1 at 5% -> 1 at 54% -> 0 at 56% -> 0; translate-y: 75px -> 0px at 5% -> 0px at 50% -> -430px at 56% -> -430px; }
  .research__result { opacity: 0 -> 0 at 54% -> 1 at 56% -> 1 at 69% -> 0 at 70% -> 0; }
  .research__offer { opacity: 0 -> 0 at 68% -> 1 at 70% -> 1; }
  .research__approved { opacity: 0 -> 0 at 85% -> 1 at 88% -> 1; scale: 0.1 -> 0.1 at 85% -> 1 at 88% -> 1; }
  .research__chat { scale: 1 -> 1 at 74% -> 2.7 at 82% -> 2.7; translate-x: 0px -> 0px at 82% -> -700px; easing: --ease-in-out-cubic; }
} }
.research__chat { transform-origin: 300px 288px; }

.feedback-chart { @on &.clip { --film-window; .feedback { scale: 1 -> 1.35 at 44% -> 1.35; translate-x: 0px -> -160px at 44% -> -160px; translate-y: 0px -> 30px at 44% -> 30px; easing: --ease-in-out-cubic; } .feedback__marker { offset-distance: 16% -> 53% at 65% -> 63%; easing: --ease-out-cubic; } .feedback__tooltip { opacity: 0 -> 0 at 20% -> 1 at 26% -> 1; scale: 0.9 -> 0.9 at 20% -> 1 at 26% -> 1; } } }

// The pointer and the visible line share one curve. Moving along it cannot
// drift off the data when its geometry changes.
.feedback__plot { --feedback-curve: path("M 0 595 C 430 270 850 145 1700 155"); }
.feedback__lightness { d: var(--feedback-curve); }
