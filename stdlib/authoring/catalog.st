/// A declaration card accepts either a reflected definition or an @data
/// declarations row. The host supplies native signature, source and docs.
@template &definition-card($d) {
  <article class="au-definition" data-definition="`$d.id`" data-kind="`$d.kind`">
    <header class="au-definition__head"><code class="au-definition__name">`$d.name`</code><span class="au-definition__kind">`$d.kind`</span></header>
    <p class="au-definition__doc">`$d.doc`</p>
    <pre class="au-definition__signature">`$d.signature`</pre>
    <footer class="au-definition__source">`$d.path`</footer>
  </article>
}
.au-definition { --authoring-surface; padding: 18px; }
.au-definition__head { display: flex; justify-content: space-between; align-items: baseline; gap: 12px; }
.au-definition__name { --authoring-code; font-size: 13px; font-weight: 600; }
.au-definition__kind { --authoring-code; color: var(--au-soft, #73796c); }
.au-definition__doc { font-size: 12px; line-height: 1.6; color: var(--au-soft, #73796c); }
.au-definition__signature { --authoring-code; white-space: pre-wrap; overflow-wrap: anywhere; line-height: 1.6; }
.au-definition__source { --authoring-code; color: var(--au-soft, #73796c); padding-top: 10px; border-top: 1px solid var(--au-line, #dedfd8); }
