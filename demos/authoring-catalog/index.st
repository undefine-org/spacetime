@import "stdlib/authoring";
@import "stdlib/film";
@import "./fixtures.st";

@font("Catalog Serif") { src: url("/assets/fonts/Tinos.ttf") format("truetype"); font-weight: 400; font-display: block; }
@data declarations $toolDeclarations from template;
@data dispatch $macroDeclarations;
@data derive $toolCards array : ST.authoring.declarationCards($toolDeclarations, "template");
@data derive $macroCards array : ST.authoring.declarationCards($macroDeclarations, "macro");

@data fetch $program : "/__spacetime/reflect.json?entry=index.st";
@data stored $selected string : "--message";
@data inline $query string : "";
@data derive $definitions array : ST.authoring.definitions($program, $query);
@data derive $properties array : ST.authoring.definitionProperties($program, $selected);
@data derive $definition object : $definitions.find(d => d.name == $selected);
@data derive $title string : ($definition ? $definition.name : $selected);
@type Example { Message | Light | Arrival }
@data derive $example Example : @match {
  ($selected == "--light-surface") => Light;
  ($selected == "--arrival") => Arrival;
  _ => Message;
}
.catalog-choice { @on &.click { $selected <- $.dataset.name; } }
.catalog-search { value <- $query; @on &.input { $query <- $.value; } }
.catalog-title { text <- $title; }
.catalog-list {
  @each($definitions as $d) {
    <button type="button" class="catalog-choice" data-name="`$d.name`"><code>`$d.name`</code><span>`$d.kind`</span></button>
  }
}
.catalog-properties { @each($properties as $p) { &property-editor($p); } }
.catalog-tool-cards { @each($toolCards as $d) { &definition-card($d); } }
.catalog-macro-cards { @each($macroCards as $d) { &definition-card($d); } }
.catalog-editor { @authoring-edit(entry: "index.st"); }
.catalog-preview {
  @state(when: $example is Message) { --example: 0; }
  @state(when: $example is Light) { --example: 1; }
  @state(when: $example is Arrival) { --example: 2; }
}
.arrival-stage { @score &.loop(3s) as &arrival { &arrival-tile at 0s for 3s; } }
.arrival-tile { @on &.clip --arrival; }
.light-stage { --light-surface; }

<div class="catalog-shell">
  <aside class="catalog-side">
    <a class="catalog-brand" href="/">Spacetime<span>Authoring library</span></a>
    <div class="catalog-side__section"><span class="catalog-kicker">01 / THE LANGUAGE</span><input class="catalog-search" type="search" placeholder="Find a definition…" aria-label="Find a definition" /><nav class="catalog-list" aria-label="Definitions"></nav></div>
    <div class="catalog-side__note"><span class="catalog-dot"></span> The source is the system.<p>Forms, templates and their tools, described by the same language.</p></div>
  </aside>
  <main class="catalog-main">
    <header class="catalog-top"><span>Library / Properties / Definition</span><span class="catalog-live"><i></i> LIVE SOURCE</span></header>
    <div class="catalog-intro"><p class="catalog-kicker">A WORKSPACE THAT KNOWS ITS LANGUAGE</p><h1>Everything has<br/>an editable <em>origin.</em></h1><p class="catalog-lede">One property editor. Different meanings. The declaration supplies the type, the default and the place to make the change.</p></div>
    <section class="catalog-workbench">
      <header class="catalog-workbench__head"><h2 class="catalog-title">--message</h2><span>Shared definition</span></header>
      <div class="catalog-workbench__body">
        <div class="catalog-preview">
          <div class="catalog-preview__label">LIVE EXAMPLE <span>01—03</span></div>
          <div class="message-stage"><--message>I found the feedback.<br/>Comfort is the clear favorite.</--message></div>
          <div class="light-stage"><div class="light-orb"></div><p>Strength changes the shared surface.</p></div>
          <div class="arrival-stage"><div class="arrival-tile"><span>80</span><small>SPACE BECOMES TIME</small></div><p>Translation + focus + visibility</p></div>
          <div class="catalog-preview__footer"><span>Compiled from fixtures.st</span><span>Native Spacetime</span></div>
        </div>
        <div class="catalog-editor"><p class="catalog-kicker">PARAMETERS</p><div class="catalog-properties"></div><p class="catalog-save" data-authoring-status>Change a value to edit its declared default.</p></div>
      </div>
    </section>
    <div class="catalog-principles"><article><span>01</span><h3>Meaning before controls</h3><p>Types choose the input. A color stays a color, wherever it is edited.</p></article><article><span>02</span><h3>Ownership stays visible</h3><p>Every parameter leads back to its definition. Shared changes remain explicit.</p></article><article><span>03</span><h3>One language, all the way</h3><p>The preview, the inspector and the catalog use the same reusable forms.</p></article></div>
    <section class="catalog-tools"><p class="catalog-kicker">02 / THE TOOLS THAT BUILT THIS PAGE</p><h2>Composed from its own vocabulary.</h2><p>These templates and macros are read from the actual declarations. The same components power the catalog and Studio.</p><div class="catalog-tool-cards"></div><div class="catalog-macro-cards"></div></section>
  </main>
</div>

:root { --brand-display: "Catalog Serif", Georgia, serif; --au-panel: #fff; --au-line: #dedfd8; --au-ink: #242820; --au-soft: #777d70; --au-accent: #d8ec9b; }
* { box-sizing: border-box; }
body { margin: 0; color: var(--au-ink); background: #f4f4ee; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
button, input { font: inherit; }
.catalog-shell { display: grid; grid-template-columns: 246px 1fr; min-height: 100vh; }
.catalog-side { border-right: 1px solid var(--au-line); padding: 35px 24px; background: #eceee6; display: flex; flex-direction: column; }
.catalog-brand { font-size: 20px; font-weight: 650; letter-spacing: -0.8px; color: inherit; text-decoration: none; }
.catalog-brand span { display: block; margin-top: 6px; font: 10px ui-monospace, monospace; letter-spacing: 0.4px; color: var(--au-soft); }
.catalog-side__section { margin-top: 65px; }
.catalog-kicker { font: 9px/1.5 ui-monospace, monospace; letter-spacing: 1.3px; color: var(--au-soft); }
.catalog-search { margin: 18px 0 16px; width: 100%; padding: 11px 10px; background: #f7f8f3; border: 1px solid #dce0d3; border-radius: 6px; font-size: 11px; }
.catalog-choice { display: flex; align-items: baseline; justify-content: space-between; text-align: left; width: 100%; border: 0; border-bottom: 1px solid #dde1d4; padding: 14px 2px; background: none; cursor: pointer; color: #353c2d; }
.catalog-choice code { font-size: 11px; }
.catalog-choice span { font-size: 9px; color: var(--au-soft); }
.catalog-choice:hover { color: #3b6016; }
.catalog-side__note { margin-top: auto; padding-top: 60px; font: 10px/1.7 ui-monospace, monospace; }
.catalog-side__note p { font: 11px/1.8 -apple-system, sans-serif; color: var(--au-soft); }
.catalog-dot, .catalog-live i { display: inline-block; width: 5px; height: 5px; margin-right: 5px; border-radius: 50%; background: #7b9847; }
.catalog-main { min-width: 0; padding: 0 54px 36px; }
.catalog-top { height: 76px; border-bottom: 1px solid var(--au-line); display: flex; align-items: center; justify-content: space-between; font: 10px ui-monospace, monospace; color: var(--au-soft); }
.catalog-live { font-size: 8px; letter-spacing: 1px; }
.catalog-intro { padding: 42px 0 32px; }
.catalog-intro h1 { font-family: var(--brand-display); font-size: clamp(42px, 4.6vw, 72px); font-weight: 400; line-height: 1.04; letter-spacing: -2.8px; margin: 18px 0; }
.catalog-intro h1 em { font-weight: 400; color: #6c7f49; }
.catalog-lede { max-width: 490px; font-size: 12px; line-height: 1.85; color: var(--au-soft); }
.catalog-workbench { border: 1px solid var(--au-line); border-radius: 12px; overflow: hidden; background: white; }
.catalog-workbench__head { padding: 17px 22px; display: flex; align-items: baseline; justify-content: space-between; border-bottom: 1px solid var(--au-line); }
.catalog-title { font: 13px ui-monospace, monospace; margin: 0; }
.catalog-workbench__head > span { color: var(--au-soft); font-size: 10px; }
.catalog-workbench__body { display: grid; grid-template-columns: 1.2fr 1fr; min-height: 360px; }
.catalog-preview { background: #e7ebdf; padding: 22px; min-width: 0; position: relative; display: flex; flex-direction: column; justify-content: center; overflow: hidden; }
.catalog-preview__label { position: absolute; top: 20px; left: 22px; right: 22px; display: flex; justify-content: space-between; font: 8px ui-monospace, monospace; letter-spacing: 1px; color: #8a9380; }
.catalog-preview__footer { position: absolute; bottom: 18px; left: 22px; right: 22px; display: flex; justify-content: space-between; font: 8px ui-monospace, monospace; color: #8a9380; }
.catalog-editor { padding: 22px; min-width: 0; }
.catalog-editor > .catalog-kicker { margin: 0 0 18px; }
.catalog-editor .au-property { padding: 14px; border-radius: 8px; }
.catalog-save { font-size: 10px; line-height: 1.6; color: var(--au-soft); }
.catalog-properties { display: grid; gap: 10px; }
.sample-message { display: flex; align-items: flex-start; gap: 12px; margin: 30px auto; max-width: 340px; background: var(--message-fill); padding: 23px; border: 1px solid #ffffffa0; border-radius: 18px; box-shadow: 0 20px 50px #35412315; }
.sample-message__avatar { display: grid; place-items: center; width: 32px; height: 32px; flex: none; border-radius: 10px; background: #517745; color: white; font: 17px Georgia, serif; }
.sample-message b { font-size: 11px; }
.sample-message p { margin: 9px 0 0; font-size: 13px; line-height: 1.65; }
.light-stage, .arrival-stage { display: none; text-align: center; }
.catalog-preview[data-st-state="Light"] .message-stage, .catalog-preview[data-st-state="Arrival"] .message-stage { display: none; }
.catalog-preview[data-st-state="Light"] .light-stage, .catalog-preview[data-st-state="Arrival"] .arrival-stage { display: block; }
.light-orb { width: 120px; height: 120px; border-radius: 50%; margin: 12px auto; background: radial-gradient(circle at 32% 27%, #fffbe8, #c5be93 40%, #57604c 78%); filter: brightness(var(--sample-intensity)); box-shadow: 0 30px 28px -20px #26321a80; }
.light-stage p, .arrival-stage p { color: #7b8670; font: 9px ui-monospace, monospace; margin-top: 22px; }
.arrival-tile { margin: 5px auto; width: 130px; height: 140px; background: #d8ec9b; border: 1px solid #a5bd71; border-radius: 20px; display: flex; flex-direction: column; justify-content: center; gap: 8px; }
.arrival-tile span { font: 64px/1 Georgia, serif; letter-spacing: -4px; }
.arrival-tile small { font: 6px ui-monospace, monospace; letter-spacing: 1px; }
.catalog-principles { display: grid; grid-template-columns: repeat(3, 1fr); gap: 35px; margin-top: 32px; }
.catalog-principles article > span { font: 9px ui-monospace, monospace; color: #98a28a; }
.catalog-principles h3 { font-size: 11px; font-weight: 600; margin: 12px 0 8px; }
.catalog-principles p { color: var(--au-soft); font-size: 11px; line-height: 1.8; margin: 0; }
.catalog-tools { margin-top: 50px; padding-top: 30px; border-top: 1px solid var(--au-line); }
.catalog-tools > h2 { font: 30px/1.2 var(--brand-display); font-weight: 400; margin: 14px 0; }
.catalog-tools > p:not(.catalog-kicker) { color: var(--au-soft); font-size: 12px; line-height: 1.8; max-width: 540px; }
.catalog-tool-cards, .catalog-macro-cards { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 14px; margin-top: 22px; }
@media (max-width: 960px) { .catalog-shell { grid-template-columns: 190px 1fr; } .catalog-side { padding: 25px 18px; } .catalog-main { padding: 0 25px 30px; } .catalog-workbench__body { grid-template-columns: 1fr; } .catalog-preview { min-height: 300px; } }
@media (max-width: 620px) { .catalog-shell { display: block; } .catalog-side { border-right: 0; border-bottom: 1px solid var(--au-line); } .catalog-side__section { margin-top: 20px; } .catalog-side__note { display: none; } .catalog-intro h1 { font-size: 48px; } .catalog-principles { grid-template-columns: 1fr; gap: 20px; } .catalog-tool-cards, .catalog-macro-cards { grid-template-columns: 1fr; } }
