# Film reference and design language

This is a reconstruction of the 79.1-second Grok Bot 1.0 launch master linked
from https://x.ai/news/grok-bot-1-0. The reference authorizes its purple opening,
white UI, Arial-compatible Arimo display type, Tinos editorial type, and the
original message/reaction emoji. Emoji are message content, never UI icons.

- All authored behavior is Spacetime. No scripts, inline event handlers or
  application JavaScript. The design library is ordinary named forms.
- `design/` owns tokens, anatomy, surfaces and reusable motion; `scenes/` owns
  content and local choreography; `score.st` owns time; `index.st` composes.
- The camera sits inside the scene; nested concerns never animate the same
  transform on the same element. Scene opacity owns the cut, camera transform
  owns framing, component transform owns its motion.
- Every image has a descriptive alt; inspect source imagery before changing it.
- Photographic reference assets are confined here, with provenance in
  `assets/PROVENANCE.org`. They are not stdlib design assets.
- Motion softness is an authored focus envelope, not physical velocity blur.
  It is bundled with each reusable move and may be changed per invocation.
- Verify browser movement with CDP. Preserve a RED run for a new motion gate.
