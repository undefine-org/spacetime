#!/usr/bin/env python3
"""Deterministic 30-second procedural sound design for the Spacetime promo."""
from pathlib import Path
import wave
import numpy as np

SR = 48_000
DURATION = 30.0
N = int(SR * DURATION)
RNG = np.random.default_rng(20260902)


def sine(freq, dur, phase=0.0):
    n = int(round(dur * SR))
    f = np.broadcast_to(np.asarray(freq, dtype=np.float64), (n,))
    return np.sin(phase + 2 * np.pi * np.cumsum(f) / SR)


def noise(dur):
    return RNG.standard_normal(int(round(dur * SR)))


def env_adsr(dur, attack=.01, decay=.08, sustain=.7, release=.15):
    n = int(round(dur * SR))
    a, d, r = (min(n, int(round(x * SR))) for x in (attack, decay, release))
    if a + d + r > n:
        scale = n / max(1, a + d + r)
        a, d, r = (int(x * scale) for x in (a, d, r))
    s = n - a - d - r
    return np.concatenate((
        np.linspace(0, 1, a, endpoint=False),
        np.linspace(1, sustain, d, endpoint=False),
        np.full(s, sustain),
        np.linspace(sustain, 0, r, endpoint=True),
    ))[:n]


def lowpass(x, cutoff):
    """One-pole lowpass; cutoff may be scalar or per-sample."""
    x = np.asarray(x, dtype=np.float64)
    c = np.broadcast_to(np.asarray(cutoff, dtype=np.float64), x.shape)
    alpha = 1.0 - np.exp(-2.0 * np.pi * np.clip(c, 10, SR * .45) / SR)
    y = np.empty_like(x)
    state = 0.0
    for i in range(len(x)):
        state += alpha[i] * (x[i] - state)
        y[i] = state
    return y


def bandpass(x, lo, hi):
    return lowpass(x, hi) - lowpass(x, lo)


def reverb(x, decay=1.8):
    """Light stereo Schroeder-style feedback-comb send."""
    x = np.asarray(x, dtype=np.float64)
    out = np.zeros((len(x) + int(decay * SR), 2))
    out[:len(x), 0] = x
    out[:len(x), 1] = x
    for ch, delays in enumerate(((1499, 2137, 2797, 3907), (1601, 2251, 3067, 4051))):
        for j, delay in enumerate(delays):
            g = .64 - j * .055
            for start in range(delay, len(out), delay):
                end = min(len(out), start + len(x))
                out[start:end, ch] += x[:end-start] * (g ** (start / delay)) * .24
    return out


def place(buf, t, sig, gain=1.0, pan=0.0):
    """Place mono or stereo signal; pan -1 left, +1 right."""
    start = int(round(t * SR))
    s = np.asarray(sig, dtype=np.float64)
    if s.ndim == 1:
        angle = (np.clip(pan, -1, 1) + 1) * np.pi / 4
        s = np.column_stack((s * np.cos(angle), s * np.sin(angle)))
    end = min(N, start + len(s))
    if end > start:
        buf[start:end] += gain * s[:end-start]


def saw(freq, dur, detune=0.0):
    n = int(round(dur * SR))
    f = np.broadcast_to(np.asarray(freq, dtype=float), (n,)) * (1 + detune)
    phase = np.mod(np.cumsum(f) / SR, 1.0)
    return 2 * phase - 1


def boom(brightness=1.0, length=1.2):
    n = int(round(.9 * SR))
    tt = np.arange(n) / SR
    freq = np.where(tt < .2, 90 * (0.5 ** (tt / .2)), 45)
    body = sine(freq, .9) * np.exp(-tt * 4.3)
    sub = sine(38, .9) * np.exp(-tt * 3.0) * .5
    crack = bandpass(noise(.12), 500, 5000 * brightness) * np.exp(-np.arange(int(.12*SR)) / (SR*.022))
    dry = body + sub
    dry[:len(crack)] += crack * .32 * brightness
    wet = reverb(dry, length)
    return wet * np.linspace(1, 0, len(wet))[:, None]


def ping(freq, dur=.35):
    t = np.arange(int(dur * SR)) / SR
    return (sine(freq, dur) + .28*sine(freq*2.01, dur) + .12*sine(freq*3.98, dur)) * np.exp(-t*10)


def main():
    mix = np.zeros((N, 2), dtype=np.float64)

    # 0.0: sub drone + very quiet filtered room tone.
    t = np.arange(N) / SR
    drone_env = np.minimum(t / 2.0, 1.0) * (.82 + .18*np.sin(2*np.pi*.11*t))
    drone = (.65*np.sin(2*np.pi*55*t) + .22*np.sin(2*np.pi*110*t)) * drone_env
    place(mix, 0, drone, .12)
    room = lowpass(noise(DURATION), 1300)
    place(mix, 0, room, .012, -.15)

    # 0.6: focus whoosh, moving band emphasis 200 -> 4 kHz.
    n = int(.5*SR)
    raw = noise(.5)
    cut = np.geomspace(200, 4000, n)
    whoosh = lowpass(raw, cut) - lowpass(raw, np.maximum(80, cut*.38))
    whoosh *= np.sin(np.linspace(0, np.pi, n))**1.4
    place(mix, .6, whoosh, .12, .2)

    # Cinematic impacts at storyboard cuts.
    place(mix, 2.2, boom(.85, 1.2), .52)
    place(mix, 6.4, boom(1.15, 1.25), .56)

    # 3.2: eight-note A-minor pentatonic grid draw-on.
    notes = [220.0, 261.63, 293.66, 329.63, 440.0, 523.25, 587.33, 659.25]
    for i, f in enumerate(notes):
        s = ping(f, .42) * env_adsr(.42, .003, .05, .45, .20)
        place(mix, 3.2 + i*(1.5/8), s, .17, -.45 + .9*(i/7))

    # 7.5-11.5: deterministic irregular typing ticks (~22).
    gaps = RNG.uniform(.09, .22, 40)
    times = 7.5 + np.cumsum(gaps)
    times = times[times < 11.5][:22]
    for i, when in enumerate(times):
        click = bandpass(noise(.012), 3000, 5200)
        click *= np.exp(-np.arange(len(click))/(SR*.0022))
        place(mix, when, click, .055, (-.25 if i % 2 else .25))

    # 11.6: hit + slowly blooming A-minor add9 shimmer chord.
    place(mix, 11.6, boom(1.0, 1.35), .61)
    chord = [110.0, 130.81, 164.81, 220.0, 246.94]
    pad_dur = 5.6
    pe = env_adsr(pad_dur, 1.5, .25, .72, 1.2)
    for i, f in enumerate(chord):
        left = lowpass(saw(f, pad_dur, -.003-i*.0002), 1450) * pe
        right = lowpass(saw(f, pad_dur, .003+i*.0002), 1550) * pe
        stereo = np.column_stack((left, right))
        place(mix, 11.6, stereo, .035)
        place(mix, 11.6, reverb((left+right)*.5, 1.4), .010)

    # 12.8: four panel pulses, one second apart.
    for i, f in enumerate((220.0, 261.63, 329.63, 392.0)):
        clk = bandpass(noise(.025), 700, 4500) * np.exp(-np.arange(int(.025*SR))/(SR*.006))
        pulse = ping(f, .42) * .75
        pulse[:len(clk)] += clk * .5
        place(mix, 12.8+i, pulse, .18, (-.45 + i*.3))

    # 16.5-18.0 convergence riser: rising noise band + sine sweep.
    n = int(1.5*SR)
    r_env = np.linspace(0, 1, n)**1.6
    sweep_f = np.geomspace(200, 1200, n)
    riser = sine(sweep_f, 1.5)*.35 + lowpass(noise(1.5), np.linspace(500, 7000, n))*.35
    place(mix, 16.5, riser*r_env, .24)

    # 18.0: biggest-so-far impact, 12Hz alternating frame ticks, tension bed.
    place(mix, 18.0, boom(1.3, 1.6), .72)
    for i, when in enumerate(np.arange(18.2, 22.8, 1/12)):
        c = bandpass(noise(.008), 1800, 7200)
        c *= np.exp(-np.arange(len(c))/(SR*.0014))
        place(mix, float(when), c, .075, -.72 if i % 2 == 0 else .72)
    n = int(4.5*SR)
    bed = saw(110, 4.5)
    bed = lowpass(bed, np.linspace(300, 3000, n)) * np.linspace(.25, 1, n)
    place(mix, 18.2, bed, .105)

    # 23.0: impact and procedural glass shatter.
    place(mix, 23.0, boom(1.2, 1.5), .67)
    shatter = np.zeros(int(.9*SR))
    for _ in range(60):
        onset = RNG.uniform(0, .4)
        dur = RNG.uniform(.05, .28)
        f = RNG.uniform(2000, 9000)
        s = sine(f, dur, RNG.uniform(0, 2*np.pi))
        s *= np.exp(-np.arange(len(s))/(SR*RNG.uniform(.015, .065)))
        pos = int(onset*SR)
        shatter[pos:pos+len(s)] += s * RNG.uniform(.08, .24)
    nt = bandpass(noise(.15), 900, 10000) * np.exp(-np.arange(int(.15*SR))/(SR*.025))
    shatter[:len(nt)] += nt*.6
    place(mix, 23.0, shatter, .35, .1)

    # 23.4-23.9: eight 30ms bitcrushed noise gates.
    glitch = RNG.integers(-12, 13, int(.03*SR)).astype(float) / 12
    glitch = np.repeat(glitch[::16], 16)[:int(.03*SR)]
    for i in range(8):
        place(mix, 23.4+i*(.5/8), glitch, .12, -.7 if i%2==0 else .7)

    # 27.0: deepest final impact, warm A-major resolution, bell shimmer.
    final = boom(.9, 2.5)
    final[:, 0] += np.pad(sine(32, 1.6)*np.exp(-np.arange(int(1.6*SR))/(SR*.65)), (0, len(final)-int(1.6*SR))) * .35
    final[:, 1] += np.pad(sine(32, 1.6)*np.exp(-np.arange(int(1.6*SR))/(SR*.65)), (0, len(final)-int(1.6*SR))) * .35
    place(mix, 27.0, final, .78)
    for i, f in enumerate((110.0, 138.59, 164.81, 220.0)):
        dur = 3.0
        e = env_adsr(dur, 2.0, .1, .75, .7)
        stereo = np.column_stack((lowpass(saw(f, dur, -.0025), 1800), lowpass(saw(f, dur, .0025), 1900))) * e[:,None]
        place(mix, 27.0, stereo, .042)
    shimmer_t = np.arange(int(3*SR))/SR
    shimmer = (sine(2600, 3)*.65 + sine(3900, 3)*.35) * (0.55+.45*np.sin(2*np.pi*3.2*shimmer_t))
    shimmer *= env_adsr(3, .12, .2, .45, 1.2)
    place(mix, 27.0, shimmer, .025, .25)

    # Exact 28.8-30.0 master fade, then soft-knee limiting and loudness trim.
    fade_start = int(28.8*SR)
    mix[fade_start:] *= np.linspace(1, 0, N-fade_start)[:,None]
    mix -= np.mean(mix, axis=0)
    mix *= .118 / np.sqrt(np.mean(mix*mix))
    mix = .82 * np.tanh(mix/.82)
    rms = np.sqrt(np.mean(mix*mix))
    mix *= .112 / rms
    peak = np.max(np.abs(mix))
    if peak >= .89:
        mix *= .89 / peak
    assert len(mix) == N and np.max(np.abs(mix)) < .95

    pcm = np.round(mix * 32767).astype('<i2')
    out = Path(__file__).resolve().parents[3] / 'scratch' / 'promo-audio' / 'promo.wav'
    out.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(out), 'wb') as wav:
        wav.setnchannels(2)
        wav.setsampwidth(2)
        wav.setframerate(SR)
        wav.writeframes(pcm.tobytes())
    print(f'wrote {out}: {N/SR:.6f}s peak={np.max(np.abs(mix)):.6f} rms={np.sqrt(np.mean(mix*mix)):.6f}')


if __name__ == '__main__':
    main()
