/// Shared projections: typed reflected params -> controls -> property views.
/// No DOM querying, filesystem, network, Studio globals or mutation lives here.
%primitive authoring-views() {
  %emit prelude-js {
    window.ST = window.ST || {};
    ST.authoring = ST.authoring || {};
    ST.authoring.declarationCards = function (rows, entity) {
      return (rows || []).filter(function (d) {
        return entity === 'macro' ? d.name.indexOf('authoring-') === 0 : String(d.source || '').indexOf('authoring/') >= 0;
      }).map(function (d) {
        var name = entity === 'template' ? '&' + d.name : d.name;
        return { id: name, name: name, kind: entity, doc: d.doc || '', path: d.source || 'stdlib/authoring', signature: d.signature || name + '(' + (d.params || '') + ')' };
      });
    };
    ST.authoring.definitions = function (r, query) {
      var q = String(query || '').toLowerCase();
      return ((r && r.definitions) || []).map(function (d) {
        var file = r.files[d.source.file];
        return Object.assign({}, d, { path: file ? file.path : '', signature: d.name + '(' + (d.signature || '') + ')', doc: d.doc || '' });
      }).filter(function (d) { return !q || (d.name + ' ' + d.kind + ' ' + d.doc).toLowerCase().indexOf(q) >= 0; });
    };
    ST.authoring.definitionProperties = function (r, name) {
      var d = ((r && r.definitions) || []).find(function (x) { return x.name === name; });
      if (!d) { return []; }
      var path = r.files[d.source.file].path;
      return d.params.filter(function (p) { return p.kind !== 'slot'; }).map(function (p) {
        var w = p.written;
        return ST.authoring.property(p, {
          key: d.id + ':' + p.name, origin: path + ' / ' + d.name, state: 'definition',
          scopes: [{ key: 'definition', label: 'Shared definition', selected: true, doc: 'Changes every use of this default' }],
          field: { file: path, start: w ? w.start : 0, end: w ? w.end : 0, expect: w ? w.text : '', at: '', before: '', drive: '' },
          diagnostic: w ? '' : 'Open source to add a default for this parameter.'
        });
      });
    };
    ST.authoring.field = function (p, extra) {
      var sc = p.scalar || {}, range = sc.range || [], raw = p.written ? p.written.text : (p.default == null ? '' : String(p.default));
      var bare = String(raw).replace(/^(["'])(.*)\1$/, '$2');
      var choices = p.choices && p.choices.length ? p.choices : null;
      var widget = choices ? 'choice' : (sc.widget || 'text');
      if (widget === 'number' && range[0] != null && range[1] != null) { widget = 'range'; }
      var compact = ['text', 'number', 'range', 'color', 'toggle', 'choice'];
      if (compact.indexOf(widget) < 0) { widget = 'text'; }
      return Object.assign({
        tpl: 'w-' + widget, label: p.name, type: p.type, doc: p.doc || '',
        value: bare, authored: raw, written: !!p.written,
        min: range[0] == null ? '' : range[0], max: range[1] == null ? '' : range[1],
        step: sc.step || 0.1, unit: sc.unit || '', hex: /^#[0-9a-f]{6}$/i.test(bare) ? bare : '#000000', on: bare === 'true',
        quote: /^"/.test(raw), file: '', start: 0, end: 0, expect: '', at: '', before: '', drive: '',
        choices: (choices || []).map(function (c) { return { word: c.word, doc: c.doc || '', on: c.word === bare }; })
      }, extra || {});
    };
    ST.authoring.property = function (p, extra) {
      var field = ST.authoring.field(p, extra && extra.field);
      return Object.assign({
        key: p.name, label: p.name, type: p.type, doc: p.doc || '', controls: [field],
        authored: field.authored, evaluated: field.value, origin: p.written ? 'Written in source' : 'Declared default',
        state: p.written ? 'authored' : 'default', scopes: [], diagnostic: ''
      }, extra || {});
    };
  }
}

%macro authoring-views {
  %scope file
  %form { @authoring-views; }
  %binds { authoring-views() }
}
@authoring-views;
