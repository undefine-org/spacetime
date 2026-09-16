/// Shared visual layer. Consumer shells may override layout without changing controls.

.st-fld-field { margin-bottom: 16px; }
.st-fld-label { display: block; font-size: 11px; letter-spacing: .03em; text-transform: uppercase; color: #8a8a8a; margin-bottom: 6px; }
.st-fld-input { width: 100%; border: 1px solid #E0E0E0; border-radius: 8px; padding: 9px 11px; font: inherit; font-size: 14px; color: #0a0a0a; outline: none; transition: border-color .15s, box-shadow .15s; }
.st-fld-input:focus { border-color: #FF0020; box-shadow: 0 0 0 3px rgba(255,0,32,.12); }
.st-fld-toggle { display: inline-flex; align-items: center; cursor: pointer; }
.st-fld-toggle input { display: none; }
.st-fld-toggle-track { width: 40px; height: 22px; border-radius: 20px; background: #d8d8d8; position: relative; transition: background .15s; }
.st-fld-toggle-track:after { content: ""; position: absolute; top: 2px; left: 2px; width: 18px; height: 18px; border-radius: 50%; background: #fff; transition: left .15s; }
.st-fld-toggle input:checked + .st-fld-toggle-track { background: #FF0020; }
.st-fld-toggle input:checked + .st-fld-toggle-track:after { left: 20px; }
.st-fld-color-control { position: relative; display: flex; align-items: center; border: 1px solid #E0E0E0; border-radius: 9px; padding-left: 6px; transition: border-color .15s, box-shadow .15s; }
.st-fld-color-control:focus-within { border-color: #FF0020; box-shadow: 0 0 0 3px rgba(255,0,32,.12); }
.st-fld-color-swatch { width: 26px; height: 26px; border: 1px solid rgba(0,0,0,.1); border-radius: 6px; padding: 0; cursor: pointer; flex: 0 0 auto; background: none; box-shadow: inset 0 0 0 1px rgba(255,255,255,.5); }
.st-fld-color-text { flex: 1; border: 0; border-radius: 0 9px 9px 0; background: none; padding: 9px 11px; font-family: ui-monospace, SFMono-Regular, monospace; font-size: 12.5px; letter-spacing: .02em; text-transform: uppercase; color: #0a0a0a; outline: none; }
.st-fld-picker-toggle { display: flex; align-items: center; padding: 0 8px; cursor: pointer; flex: 0 0 auto; }
.st-fld-picker-open { position: absolute; opacity: 0; width: 0; height: 0; }
.st-fld-picker-caret { width: 0; height: 0; border-left: 4px solid transparent; border-right: 4px solid transparent; border-top: 5px solid #b0b0b0; transition: transform .15s; }
.st-fld-picker-toggle:hover .st-fld-picker-caret { border-top-color: #FF0020; }
.st-fld-picker-panel { display: none; flex-direction: column; gap: 9px; margin-top: 10px; padding: 12px; border: 1px solid #ececec; border-radius: 10px; background: #fcfcfc; }
.st-fld-field--color:has(.st-fld-picker-open:checked) .st-fld-picker-panel { display: flex; }
.st-fld-field--color:has(.st-fld-picker-open:checked) .st-fld-picker-caret { transform: rotate(180deg); }
.st-fld-picker { width: 100%; height: 16px; cursor: pointer; border-radius: 8px; -webkit-appearance: none; appearance: none; }
.st-fld-picker-h { background: linear-gradient(to right, #f00 0%, #ff0 17%, #0f0 33%, #0ff 50%, #00f 67%, #f0f 83%, #f00 100%); }
.st-fld-picker::-webkit-slider-thumb { -webkit-appearance: none; width: 14px; height: 14px; border-radius: 50%; background: #fff; border: 2px solid rgba(0,0,0,.35); box-shadow: 0 1px 3px rgba(0,0,0,.25); cursor: pointer; }
.st-fld-picker::-moz-range-thumb { width: 14px; height: 14px; border-radius: 50%; background: #fff; border: 2px solid rgba(0,0,0,.35); cursor: pointer; }
.st-fld-field--range .st-fld-label { display: flex; justify-content: space-between; align-items: baseline; }
.st-fld-range-value { font-family: ui-monospace, SFMono-Regular, monospace; font-size: 12px; color: #FF0020; text-transform: none; letter-spacing: 0; }
.st-fld-range { width: 100%; accent-color: #FF0020; height: 22px; cursor: pointer; }
