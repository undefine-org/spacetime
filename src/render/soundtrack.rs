//! The film's soundtrack (PLAN-152): a score's `@sound` notes, synthesized
//! into a WAV for the renderers to mux.
//!
//! The synthesizer is ONE file, `stdlib/film/synth/synth.js` — the same bytes a
//! browser runs live (the `score-synth` primitive) — executed here in V8. So a
//! website and its rendered video hear the same samples, and neither engine
//! needs a browser to make them: Chromium's mux takes the WAV as ffmpeg's
//! second input, raea's takes it as the page's `<audio>`.

use std::path::Path;

use serde_json::Value;

/// The synthesizer, embedded so a distributed binary carries it (the file in
/// stdlib is the one source).
const SYNTH: &str = include_str!("../../stdlib/film/synth/synth.js");

/// One program from a page's scores (most films have one): every score's
/// sounds and forms, the longest score's length.
fn merged(programs: &[Value]) -> Option<Value> {
    let first = programs.first()?;
    if programs.len() == 1 {
        return Some(first.clone());
    }
    let mut sounds = Vec::new();
    let mut forms = serde_json::Map::new();
    let mut total = 0.0f64;
    for p in programs {
        sounds.extend(p["sounds"].as_array().cloned().unwrap_or_default());
        if let Some(f) = p["forms"].as_object() {
            forms.extend(f.clone());
        }
        total = total.max(p["total"].as_f64().unwrap_or(0.0));
    }
    Some(serde_json::json!({ "total": total, "sounds": sounds, "forms": forms }))
}

/// Write the soundtrack for `[from_ms, to_ms)` of the score to `out` (48 kHz,
/// 16-bit stereo WAV). `Ok(false)` when the page has no `@sound`.
pub(crate) fn write_soundtrack(programs: &[Value], from_ms: u64, to_ms: u64, out: &Path) -> Result<bool, String> {
    let Some(program) = merged(programs) else { return Ok(false) };
    let b64 = synthesize(&program, from_ms, to_ms)?;
    let bytes = decode_base64(&b64)?;
    std::fs::write(out, bytes).map_err(|e| format!("failed to write {}: {e}", out.display()))?;
    Ok(true)
}

#[cfg(feature = "headless")]
fn synthesize(program: &Value, from_ms: u64, to_ms: u64) -> Result<String, String> {
    let code = format!(
        "{SYNTH}\nexport const result = globalThis.STSynth.wav({program}, {{ sr: 48000, from: {from_ms}, to: {to_ms} }});"
    );
    // V8 runtimes are thread-bound; run on a thread of its own, as the SSG
    // filters do (html/ssg_filters.rs).
    std::thread::spawn(move || -> Result<String, String> {
        use rustyscript::{Module, Runtime, RuntimeOptions};
        crate::ensure_v8_initialized();
        let mut rt = Runtime::new(RuntimeOptions::default()).map_err(|e| format!("the synthesizer's V8 didn't start: {e}"))?;
        let module = Module::new("st_synth.js", &code);
        let handle = rt.load_module(&module).map_err(|e| format!("the synthesizer failed: {e}"))?;
        rt.get_value::<String>(Some(&handle), "result").map_err(|e| format!("the synthesizer returned no WAV: {e}"))
    })
    .join()
    .map_err(|_| "the synthesizer panicked".to_string())?
}

#[cfg(not(feature = "headless"))]
fn synthesize(_program: &Value, _from_ms: u64, _to_ms: u64) -> Result<String, String> {
    Err("this film has `@sound`, which needs the `headless` feature (V8) to synthesize its soundtrack".into())
}

fn decode_base64(s: &str) -> Result<Vec<u8>, String> {
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    };
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
        if chunk.len() < 4 {
            return Err("the synthesizer's WAV was cut short".into());
        }
        let a = val(chunk[0]).ok_or("bad base64")?;
        let b = val(chunk[1]).ok_or("bad base64")?;
        out.push(((a << 2) | (b >> 4)) as u8);
        if chunk[2] != b'=' {
            let c = val(chunk[2]).ok_or("bad base64")?;
            out.push((((b & 15) << 4) | (c >> 2)) as u8);
            if chunk[3] != b'=' {
                let d = val(chunk[3]).ok_or("bad base64")?;
                out.push((((c & 3) << 6) | d) as u8);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A beat placed on a score becomes a WAV with sound in its window and
    /// silence before it — synthesized by the same file the browser runs.
    #[cfg(feature = "headless")]
    #[test]
    fn a_pattern_on_the_score_becomes_a_wav() {
        let program = serde_json::json!({
            "total": 2000.0,
            "sounds": [{ "form": "--beat", "at": 1000.0, "dur": 1000.0, "notes": null, "name": "" }],
            "forms": {
                "--kick": { "kind": "instrument", "params": {}, "props": { "wave": ["sine"], "pitch": ["160hz", "45hz at 30%"], "gain": ["1", "0"], "for": ["300ms"] } },
                "--beat": { "kind": "pattern", "params": { "tempo": "120bpm", "step": "16" }, "props": { "--kick": ["x . . ."] } }
            }
        });
        let out = std::env::temp_dir().join(format!("st_soundtrack_{}.wav", std::process::id()));
        assert!(write_soundtrack(&[program], 0, 2000, &out).unwrap());
        let wav = std::fs::read(&out).unwrap();
        std::fs::remove_file(&out).ok();
        assert_eq!(&wav[0..4], b"RIFF");
        let samples: Vec<i16> = wav[44..].chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect();
        let frames = samples.len() / 2;
        assert_eq!(frames, 96_000, "2 s at 48 kHz");
        let rms = |a: usize, b: usize| (samples[a * 2..b * 2].iter().map(|&s| (s as f64).powi(2)).sum::<f64>() / ((b - a) * 2) as f64).sqrt();
        assert!(rms(0, 48_000) < 1.0, "silence before the sound's window");
        assert!(rms(48_000, 96_000) > 1000.0, "the beat is heard in its window");
    }
}
