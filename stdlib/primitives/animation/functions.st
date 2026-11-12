/// Animation Runtime Functions
///
/// Provides dynamic value generators for animations:
/// - wave: Oscillating values (sin, cos, triangle, etc.)
/// - random: Seeded random values
/// - noise: Perlin-like noise
/// - parallax: Scroll-based position offsets


/// Wave Function Primitive
///
/// Generates oscillating values based on progress.

%primitive wave(
  /// Wave type: "sin", "cos", "triangle", "sawtooth", "square"
  type: string = "sin",
  /// Number of cycles per progress unit
  frequency: number = 1,
  /// Peak amplitude (can include unit like "20px")
  amplitude: string = "1",
  /// Phase offset in radians
  phase: number = 0
) {
  %emit js {
    const waveType = '%type';
    const freq = %frequency;
    const ampStr = '%amplitude';
    const ph = %phase;

    // Parse amplitude with unit
    const ampMatch = String(ampStr).match(/^(-?\d*\.?\d+)(.*)$/);
    const amp = ampMatch ? parseFloat(ampMatch[1]) : 1;
    const unit = ampMatch ? ampMatch[2] : '';

    const getValue = (progress) => {
      const t = progress * freq * Math.PI * 2 + ph;
      let value;

      switch (waveType) {
        case 'sin':
          value = Math.sin(t);
          break;
        case 'cos':
          value = Math.cos(t);
          break;
        case 'triangle':
          value = 2 * Math.abs(2 * ((t / (2 * Math.PI)) % 1) - 1) - 1;
          break;
        case 'sawtooth':
          value = 2 * ((t / (2 * Math.PI)) % 1) - 1;
          break;
        case 'square':
          value = Math.sin(t) >= 0 ? 1 : -1;
          break;
        default:
          value = Math.sin(t);
      }

      const result = amp * value;
      return unit ? result + unit : result;
    };

    %yield getValue -> $getValue;
  }

  %exports {
    /// Function that takes progress and returns wave value
    $getValue: fn(progress: number) ~> any
  }
}


/// Random Function Primitive
///
/// Generates seeded random values for consistent per-element randomness.

%primitive random(
  /// Minimum value (can include unit)
  min: string = "0",
  /// Maximum value (can include unit)
  max: string = "1",
  /// Seed for random generator
  seed: number = 1
) {
  %emit js {
    const minStr = '%min';
    const maxStr = '%max';
    const s = %seed;

    // Parse values with units
    const parseWithUnit = (str) => {
      const match = String(str).match(/^(-?\d*\.?\d+)(.*)$/);
      return match ? { num: parseFloat(match[1]), unit: match[2] } : { num: 0, unit: '' };
    };

    const minParsed = parseWithUnit(minStr);
    const maxParsed = parseWithUnit(maxStr);
    const unit = maxParsed.unit || minParsed.unit;

    // Seeded random using sine
    const x = Math.sin(s) * 10000;
    const rand = x - Math.floor(x);

    const result = minParsed.num + (maxParsed.num - minParsed.num) * rand;
    const value = unit ? result + unit : result;

    %yield value -> $value;
  }

  %exports {
    /// Random value between min and max
    $value: any
  }
}


/// Noise Function Primitive
///
/// Generates Perlin-like noise based on position and progress.

%primitive noise(
  /// Element for position-based noise
  &el,
  /// Noise frequency
  frequency: number = 1,
  /// Noise amplitude (can include unit)
  amplitude: string = "1"
) {
  %emit js {
    const freq = %frequency;
    const ampStr = '%amplitude';

    const ampMatch = String(ampStr).match(/^(-?\d*\.?\d+)(.*)$/);
    const amp = ampMatch ? parseFloat(ampMatch[1]) : 1;
    const unit = ampMatch ? ampMatch[2] : '';

    const getValue = (progress) => {
      const rect = %&el.getBoundingClientRect();
      const x = rect.left / 100;
      const y = rect.top / 100;

      const nx = (x + progress) * freq;
      const ny = y * freq;

      // Simple Perlin-like noise using layered sine waves
      const noise =
        Math.sin(nx) * Math.cos(ny) * 0.5 +
        Math.sin(nx * 2.1 + 1.3) * Math.cos(ny * 1.9 + 2.1) * 0.25 +
        Math.sin(nx * 3.7 + 2.7) * Math.cos(ny * 2.3 + 3.7) * 0.125;

      const result = amp * noise;
      return unit ? result + unit : result;
    };

    %yield getValue -> $getValue;
  }

  %exports {
    /// Function that takes progress and returns noise value
    $getValue: fn(progress: number) ~> any
  }
}


/// Parallax Function Primitive
///
/// Maps progress to position offset for parallax effects.

%primitive parallax(
  /// Total movement range (can include unit like "100px")
  range: string = "0px",
  /// Center offset position
  center: string = "0px"
) {
  %emit js {
    const rangeStr = '%range';
    const centerStr = '%center';

    const parseWithUnit = (str) => {
      const match = String(str).match(/^(-?\d*\.?\d+)(.*)$/);
      return match ? { num: parseFloat(match[1]), unit: match[2] } : { num: 0, unit: '' };
    };

    const rangeParsed = parseWithUnit(rangeStr);
    const centerParsed = parseWithUnit(centerStr);
    const unit = rangeParsed.unit || centerParsed.unit;

    const getValue = (progress) => {
      const result = (progress - 0.5) * rangeParsed.num + centerParsed.num;
      return unit ? result + unit : result;
    };

    %yield getValue -> $getValue;
  }

  %exports {
    /// Function that takes progress and returns parallax offset
    $getValue: fn(progress: number) ~> any
  }
}


/// Spring Function Primitive
///
/// Generates spring-physics based motion.

%primitive spring(
  /// Spring stiffness (higher = faster)
  stiffness: number = 100,
  /// Damping ratio (0-1, 1 = critically damped)
  damping: number = 10,
  /// Mass of the object
  mass: number = 1
) {
  %emit js {
    const k = %stiffness;
    const c = %damping;
    const m = %mass;

    // Pre-calculate spring constants
    const omega = Math.sqrt(k / m);
    const zeta = c / (2 * Math.sqrt(k * m));

    const getValue = (progress) => {
      if (progress >= 1) return 1;
      if (progress <= 0) return 0;

      // Approximate spring motion
      const t = progress * 4; // Scale time

      if (zeta < 1) {
        // Underdamped
        const omegaD = omega * Math.sqrt(1 - zeta * zeta);
        const decay = Math.exp(-zeta * omega * t);
        return 1 - decay * (Math.cos(omegaD * t) + (zeta * omega / omegaD) * Math.sin(omegaD * t));
      } else if (zeta === 1) {
        // Critically damped
        const decay = Math.exp(-omega * t);
        return 1 - decay * (1 + omega * t);
      } else {
        // Overdamped
        const s1 = omega * (-zeta + Math.sqrt(zeta * zeta - 1));
        const s2 = omega * (-zeta - Math.sqrt(zeta * zeta - 1));
        const decay1 = Math.exp(s1 * t);
        const decay2 = Math.exp(s2 * t);
        return 1 - (s2 * decay1 - s1 * decay2) / (s2 - s1);
      }
    };

    %yield getValue -> $getValue;
  }

  %exports {
    /// Function that takes progress and returns spring value
    $getValue: fn(progress: number) ~> number
  }
}
