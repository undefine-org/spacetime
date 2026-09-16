//! AST to .st source text serializer
//!
//! Converts a parsed StFile AST back to .st source code for round-trip editing.
//! Note: Since timelines are now created via macros, this serializer focuses on
//! presets, imports, and basic scope structure.

use crate::parser::{
    AnimationProperty, ConfigArg, ConfigValue, EasingDef, EasingValue, PresetDef, PresetType,
    PresetValue, ScopeBlock, StFile, StaggerDirection, Value,
};

/// Serialize an StFile AST back to .st source text
pub fn serialize(file: &StFile) -> String {
    let mut output = String::new();

    // Imports
    for import in &file.imports {
        output.push_str(&format!("@import \"{}\";\n", import.path));
    }
    if !file.imports.is_empty() {
        output.push('\n');
    }

    // Presets
    for preset in &file.presets {
        serialize_preset(&mut output, preset);
    }
    if !file.presets.is_empty() {
        output.push('\n');
    }

    // Scopes (simplified - only outputs selector and macro calls)
    for scope in &file.scopes {
        serialize_scope(&mut output, scope);
        output.push('\n');
    }

    output
}

fn serialize_preset(output: &mut String, preset: &PresetDef) {
    let type_str = match preset.preset_type {
        PresetType::Easing => "easing",
        PresetType::Scroll => "scroll",
        PresetType::Animation => "animation",
        PresetType::Load => "load",
    };

    output.push_str(&format!("@preset {} {}: ", type_str, preset.name));

    match &preset.value {
        PresetValue::Easing(easing) => {
            serialize_easing_value(output, easing);
            output.push_str(";\n");
        }
        PresetValue::Config(args) => {
            serialize_config_args(output, args);
            output.push_str(";\n");
        }
        PresetValue::AnimationBlock(props) => {
            output.push_str("{\n");
            for prop in props {
                serialize_animation_property(output, prop, 1);
            }
            output.push_str("}\n");
        }
    }
}

fn serialize_scope(output: &mut String, scope: &ScopeBlock) {
    output.push_str(&format!("{} {{\n", scope.selector));

    // Serialize directives from FormMatches
    for fm in &scope.matches {
        output.push_str(&format!("    @{}", fm.macro_name));
        if !fm.captures.is_empty() {
            output.push_str("(...)"); // Simplified
        }
        output.push_str(";\n");
    }

    // Serialize CSS declarations
    for css in &scope.css_declarations {
        output.push_str(&format!("    {}: {};\n", css.property, css.value));
    }

    output.push_str("}\n");
}

fn serialize_config_value(value: &ConfigValue) -> String {
    match value {
        ConfigValue::Number(n) => format!("{}", n),
        ConfigValue::String(s) => s.clone(),
        ConfigValue::Selector(s) => s.clone(),
        ConfigValue::Preset(p) => p.clone(),
    }
}

fn serialize_config_args(output: &mut String, args: &[ConfigArg]) {
    let parts: Vec<String> = args
        .iter()
        .map(|arg| format!("{}: {}", arg.key, serialize_config_value(&arg.value)))
        .collect();
    output.push_str(&parts.join(", "));
}

fn serialize_animation_property(output: &mut String, prop: &AnimationProperty, indent: usize) {
    let indent_str = "    ".repeat(indent);

    match prop {
        AnimationProperty::Apply(preset_names) => {
            output.push_str(&format!(
                "{}apply: {};\n",
                indent_str,
                preset_names.join(", ")
            ));
        }
        AnimationProperty::Transition(trans) => {
            output.push_str(&format!("{}{}: ", indent_str, trans.property));

            let values: Vec<String> = trans.values.iter().map(serialize_value).collect();
            output.push_str(&values.join(" -> "));

            if let Some(easing) = &trans.inline_easing {
                output.push_str(" with ");
                serialize_easing_def(output, easing);
            }

            output.push_str(";\n");
        }
        AnimationProperty::Keyframes(kf) => {
            output.push_str(&format!("{}{}: {{\n", indent_str, kf.property));
            for keyframe in &kf.keyframes {
                output.push_str(&format!(
                    "{}    {}%: {};\n",
                    indent_str,
                    keyframe.percentage,
                    serialize_value(&keyframe.value)
                ));
            }
            output.push_str(&format!("{}}};\n", indent_str));
        }
        AnimationProperty::Easing(easing) => {
            output.push_str(&format!("{}easing: ", indent_str));
            serialize_easing_def(output, easing);
            output.push_str(";\n");
        }
        AnimationProperty::Timing { offset, span } => {
            output.push_str(&format!("{}range: {}", indent_str, offset));
            if let Some(s) = span {
                output.push_str(&format!(" + {}", s));
            }
            output.push_str(";\n");
        }
        AnimationProperty::Stagger(stagger) => {
            output.push_str(&format!("{}stagger: {}", indent_str, stagger.delay));
            if let Some((cols, rows)) = stagger.grid {
                output.push_str(&format!(" grid({} {})", cols, rows));
            }
            output.push_str(&format!(
                " {};\n",
                serialize_stagger_direction(&stagger.direction)
            ));
        }
        AnimationProperty::ColorSpace(space) => {
            output.push_str(&format!("{}color-space: {};\n", indent_str, space));
        }
        AnimationProperty::Static(static_prop) => {
            output.push_str(&format!(
                "{}{}: {};\n",
                indent_str,
                static_prop.property,
                serialize_value(&static_prop.value)
            ));
        }
    }
}

fn serialize_easing_def(output: &mut String, easing: &EasingDef) {
    serialize_easing_value(output, &easing.value);
}

fn serialize_easing_value(output: &mut String, value: &EasingValue) {
    match value {
        EasingValue::Preset(name) => {
            output.push_str(name);
        }
        EasingValue::CubicBezier(a, b, c, d) => {
            output.push_str(&format!("cubic-bezier({}, {}, {}, {})", a, b, c, d));
        }
        EasingValue::Spring {
            stiffness,
            damping,
            mass,
        } => {
            output.push_str(&format!("spring({}, {}, {})", stiffness, damping, mass));
        }
    }
}

fn serialize_value(value: &Value) -> String {
    match value {
        Value::Number(n) => format!("{}", n),
        Value::NumberWithUnit(n, unit) => format!("{}{}", n, unit),
        Value::Color(c) => c.clone(),
        Value::Calc(expr) => format!("calc({})", expr),
        Value::String(s) => format!("\"{}\"", s),
        Value::Identifier(id) => id.clone(),
        Value::CssFunction(name, content) => format!("{}({})", name, content),
        Value::FunctionCall(name, args) => {
            let arg_strs: Vec<String> = args.iter().map(serialize_value).collect();
            format!("{}({})", name, arg_strs.join(", "))
        }
        Value::ElementRef {
            name,
            facet,
            property,
        } => {
            format!("&{}.{}.{}", name, facet, property)
        }
    }
}

fn serialize_stagger_direction(dir: &StaggerDirection) -> String {
    match dir {
        StaggerDirection::First => "first".to_string(),
        StaggerDirection::Last => "last".to_string(),
        StaggerDirection::Center => "center".to_string(),
        StaggerDirection::Random => "random".to_string(),
        StaggerDirection::Index(i) => format!("index({})", i),
    }
}

#[cfg(test)]
mod tests {}
