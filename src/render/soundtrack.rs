//! The film's soundtrack: recorded `@audio` clips and synthesized `@sound`
//! notes, placed in sample time in one WAV for both renderers to mux.
//!
//! The synthesizer is ONE file, `stdlib/film/synth/synth.js` — the same bytes a
//! browser runs live (the `score-synth` primitive) — executed here in V8. So a
//! website and its rendered video hear the same samples, and neither engine
//! needs a browser to make them: Chromium's mux takes the WAV as ffmpeg's
//! second input, raea's takes it as the page's `<audio>`.

use std::path::Path;
use std::process::Command;

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
    let mut audio = Vec::new();
    let mut forms = serde_json::Map::new();
    let mut total = 0.0f64;
    for p in programs {
        sounds.extend(p["sounds"].as_array().cloned().unwrap_or_default());
        audio.extend(p["audio"].as_array().cloned().unwrap_or_default());
        if let Some(f) = p["forms"].as_object() {
            forms.extend(f.clone());
        }
        total = total.max(p["total"].as_f64().unwrap_or(0.0));
    }
    Some(serde_json::json!({ "total": total, "sounds": sounds, "forms": forms, "audio": audio }))
}

/// Write the soundtrack for `[from_ms, to_ms)` of the score to `out` (48 kHz,
/// 16-bit stereo WAV). `Ok(false)` when the page has no sound. Recorded clips
/// use the existing FFmpeg dependency; only synth notes require the V8 feature.
pub(crate) fn write_soundtrack(
    programs: &[Value],
    site_dir: &Path,
    from_ms: u64,
    to_ms: u64,
    out: &Path,
) -> Result<bool, String> {
    let Some(program) = merged(programs) else {
        return Ok(false);
    };
    if to_ms <= from_ms {
        return Err("the soundtrack window must have positive duration".into());
    }
    let audio = program["audio"].as_array().cloned().unwrap_or_default();
    let has_notes = program["sounds"].as_array().is_some_and(|s| !s.is_empty());
    if audio.is_empty() {
        if !has_notes {
            return Ok(false);
        }
        write_synth(&program, from_ms, to_ms, out)?;
    } else {
        mix_recordings(&program, &audio, has_notes, site_dir, from_ms, to_ms, out)?;
    }
    Ok(true)
}

fn write_synth(program: &Value, from_ms: u64, to_ms: u64, out: &Path) -> Result<(), String> {
    let bytes = decode_base64(&synthesize(program, from_ms, to_ms)?)?;
    std::fs::write(out, bytes).map_err(|e| format!("failed to write {}: {e}", out.display()))
}

/// Decode, place and mix recordings. The silence input defines the exact
/// window length: a short clip must never truncate the picture, and a window
/// before a recording still has its requested duration. All generated filter
/// expressions contain numbers only; source paths are separate process args.
fn mix_recordings(
    program: &Value,
    audio: &[Value],
    has_notes: bool,
    site_dir: &Path,
    from_ms: u64,
    to_ms: u64,
    out: &Path,
) -> Result<(), String> {
    let scratch = tempfile::tempdir().map_err(|e| format!("failed to stage audio: {e}"))?;
    let frames = (to_ms - from_ms) * 48;
    let duration = (to_ms - from_ms) as f64 / 1000.0;
    let mut inputs: Vec<(std::ffi::OsString, f64)> = Vec::new();
    if has_notes {
        let synth = scratch.path().join("notes.wav");
        write_synth(program, from_ms, to_ms, &synth)?;
        // The synth WAV already represents this window, not score time zero.
        inputs.push((synth.into_os_string(), from_ms as f64));
    }
    for clip in audio {
        let src = clip["src"]
            .as_str()
            .ok_or("an audio clip has no source URL")?;
        let at = clip["at"].as_f64().unwrap_or(0.0);
        if !at.is_finite() {
            return Err(format!("invalid audio start for {src}"));
        }
        if at >= to_ms as f64 {
            continue;
        }
        let source = if src.contains("://") || src.starts_with("data:") {
            std::ffi::OsString::from(src)
        } else {
            site_dir.join(src.trim_start_matches('/')).into_os_string()
        };
        inputs.push((source, at));
    }

    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-y", "-v", "error", "-f", "lavfi", "-i"])
        .arg(format!("anullsrc=r=48000:cl=stereo:d={duration:.9}"));
    let mut filters = vec![format!(
        "[0:a]atrim=end_sample={frames},asetpts=PTS-STARTPTS[a0]"
    )];
    for (i, (source, at)) in inputs.iter().enumerate() {
        cmd.arg("-i").arg(source);
        let index = i + 1;
        let trim = (((from_ms as f64 - at).max(0.0)) * 48.0).round() as u64;
        let delay = (((at - from_ms as f64).max(0.0)) * 48.0).round() as u64;
        filters.push(format!("[{index}:a]aresample=48000,aformat=sample_fmts=fltp:channel_layouts=stereo,atrim=start_sample={trim},asetpts=PTS-STARTPTS,adelay={delay}S:all=1,apad=whole_len={frames},atrim=end_sample={frames}[a{index}]"));
    }
    let tracks = inputs.len() + 1;
    let labels: String = (0..tracks).map(|i| format!("[a{i}]")).collect();
    filters.push(format!(
        "{labels}amix=inputs={tracks}:duration=longest:normalize=0,atrim=end_sample={frames}[mix]"
    ));
    let result = cmd
        .arg("-filter_complex")
        .arg(filters.join(";"))
        .args([
            "-map",
            "[mix]",
            "-ar",
            "48000",
            "-ac",
            "2",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(out)
        .output()
        .map_err(|e| format!("FFmpeg is required to render recorded audio: {e}"))?;
    if !result.status.success() {
        return Err(format!(
            "failed to render recorded audio: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }
    Ok(())
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
        let mut rt = Runtime::new(RuntimeOptions::default())
            .map_err(|e| format!("the synthesizer's V8 didn't start: {e}"))?;
        let module = Module::new("st_synth.js", &code);
        let handle = rt
            .load_module(&module)
            .map_err(|e| format!("the synthesizer failed: {e}"))?;
        rt.get_value::<String>(Some(&handle), "result")
            .map_err(|e| format!("the synthesizer returned no WAV: {e}"))
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

    #[cfg(feature = "cdp")]
    #[test]
    fn soundtrack_metadata_uses_the_existing_audio_and_sound_captures() {
        let entry =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/render/fixtures/file_audio.st");
        let (_, total, programs) =
            super::super::compile_page(&entry, entry.parent().unwrap(), ".").unwrap();
        assert_eq!(total, 3000);
        assert_eq!(programs.len(), 1);
        assert_eq!(
            programs[0]["audio"],
            serde_json::json!([
                { "src": "audio/two tones.flac", "name": "recording", "at": 1000.0 }
            ])
        );
        assert_eq!(programs[0]["sounds"].as_array().unwrap().len(), 1);
    }

    #[cfg(feature = "cdp")]
    #[test]
    fn identical_score_text_in_distinct_modules_is_not_deduplicated() {
        let dir = tempfile::tempdir().unwrap();
        let source = r#".film {
            @score &.time(1s) {
                &picture at 0s for 1s;
                @audio(src: "voice.flac") as &voice at 0s;
            }
        }"#;
        for name in ["a.st", "b.st"] {
            std::fs::write(dir.path().join(name), source).unwrap();
        }
        let entry = dir.path().join("index.st");
        std::fs::write(
            &entry,
            r#"@import "./a.st"; @import "./b.st";
            <div class="film"><div class="picture">Picture</div></div>"#,
        )
        .unwrap();
        let (_, _, programs) = super::super::compile_page(&entry, dir.path(), ".").unwrap();
        assert_eq!(
            programs.len(),
            2,
            "different source files own different scores even at identical spans"
        );
        assert_eq!(
            merged(&programs).unwrap()["audio"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }

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
        assert!(write_soundtrack(&[program], Path::new("."), 0, 2000, &out).unwrap());
        let wav = std::fs::read(&out).unwrap();
        std::fs::remove_file(&out).ok();
        assert_eq!(&wav[0..4], b"RIFF");
        let samples: Vec<i16> = wav[44..]
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
            .collect();
        let frames = samples.len() / 2;
        assert_eq!(frames, 96_000, "2 s at 48 kHz");
        let rms = |a: usize, b: usize| {
            (samples[a * 2..b * 2]
                .iter()
                .map(|&s| (s as f64).powi(2))
                .sum::<f64>()
                / ((b - a) * 2) as f64)
                .sqrt()
        };
        assert!(rms(0, 48_000) < 1.0, "silence before the sound's window");
        assert!(
            rms(48_000, 96_000) > 1000.0,
            "the beat is heard in its window"
        );
    }
}
