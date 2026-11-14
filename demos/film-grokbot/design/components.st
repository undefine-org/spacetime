/// All markup is reusable by name. Slots compose without a second component
/// language: a scene contains a camera, a camera contains ordinary forms.
@form markup --scene(&content, $name string, $tone string = "paper") {
  <section class="scene scene--`$tone` `$name`"><div class="scene__camera">`&content`</div></section>
}
@form markup --bot($kind string = "blue", $label string = "Research Bot") {
  <span class="bot bot--`$kind`" role="img" aria-label="`$label`"><span class="bot__body"></span><span class="bot__eyes"><i></i><i></i></span></span>
}
@form markup --icon($name string, $label string = "Control") {
  <span class="icon"><img src="assets/icons/`$name`.svg" alt="`$label`" /></span>
}
@form markup --wave {
  <span class="wave" aria-label="Voice activity"><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i></span>
}
@form markup --voice(&content, $kind string = "blue") {
  <div class="voice voice--`$kind`"><div class="voice__bar"><--bot kind="`$kind`" /><--wave /><span class="voice__spacer"></span><--icon name="settings" label="Voice settings" /><--icon name="message-circle" label="Open conversation" /><--icon name="mic" label="Microphone" /><span class="voice__end"><--icon name="x" label="End call" /></span></div><div class="voice__conversation">`&content`</div></div>
}
@form markup --bubble(&content, $side string = "in") {
  <div class="bubble-row bubble-row--`$side`"><div class="bubble bubble--`$side`">`&content`</div></div>
}
@form markup --message(&content, $name string = "Research Bot", $kind string = "blue", $time string = "10:02 AM", $app string = "") {
  <article class="message"><div class="message__avatar"><--bot kind="`$kind`" label="`$name`" /></div><div class="message__main"><header class="message__header"><b>`$name`</b><small class="message__app">`$app`</small><time>`$time`</time></header><div class="message__content">`&content`</div></div></article>
}
@form markup --pointer($kind string = "blue") {
  <span class="pointer pointer--`$kind`"><span class="pointer__arrow"></span><span class="pointer__avatar"><--bot kind="`$kind`" /></span></span>
}
@form markup --reaction(&content) { <span class="reaction">`&content`</span> }
@form markup --composer {
  <div class="composer"><--icon name="plus" label="Add to conversation" /><span>History is on</span><--icon name="type" label="Format message" /><--icon name="ellipsis" label="More actions" /><--icon name="send" label="Send message" /></div>
}
@form markup --toolstrip {
  <div class="toolstrip"><span class="toolstrip__active"><--icon name="mouse-pointer-2" label="Select" /></span><--icon name="frame" label="Frame" /><--icon name="square" label="Shape" /><--icon name="pen-tool" label="Pen" /><--icon name="type" label="Text" /><--icon name="message-circle" label="Comment" /><--icon name="code" label="Code" /></div>
}
/// A deterministic photographic filmstrip: CSS lays out atlas cells; a score
/// animates --frame. It uses the same clock as every other property, so a
/// still, reverse seek and export all show the same photographic frame.
@form markup --filmstrip($src string, $alt string, $columns number = 10, $rows number = 12, $aspect number = 1.9246231156) {
  <div class="filmstrip" style="--columns: `$columns`; --rows: `$rows`; --aspect: `$aspect`;"><img src="`$src`" alt="`$alt`" /></div>
}
@form markup --participant(&content, $name string) {
  <div class="participant">`&content`<span class="participant__name">`$name`</span></div>
}
@form markup --poster($src string, $alt string, $format string = "portrait") {
  <article class="poster poster--`$format`"><img src="`$src`" alt="`$alt`" /><span class="poster__brand">Pendel Arc</span><span class="poster__line">A lighter fit for longer days.</span></article>
}
