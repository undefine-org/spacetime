//! CLI commands and utilities

pub mod doctor;
pub mod export_profile;
pub mod generate_grammar;
pub mod inspect;

pub use doctor::{DoctorArgs, DoctorReport, run_doctor};
pub use generate_grammar::{GenerateGrammarArgs, run as run_generate_grammar};
pub use inspect::{
    DispatchCandidate, DispatchProbe, InspectFilter, InspectLayer, OutputFormat,
    build_dispatch_probe, run_inspect,
};
