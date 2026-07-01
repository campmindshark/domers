//! Frame-render microbenchmark used to size hardware for domers.
//!
//! Run with: `cargo run --release --example bench_frame -- examples/domers.toml`

use std::fs;
use std::time::Instant;

use domers_core::DomersConfig;
use domers_server::ServerState;

fn bench<F: FnMut() -> usize>(label: &str, iters: u32, mut f: F) {
    // warmup
    let mut sink = 0usize;
    for _ in 0..(iters / 10).max(1000) {
        sink = sink.wrapping_add(f());
    }
    let start = Instant::now();
    for _ in 0..iters {
        sink = sink.wrapping_add(f());
    }
    let dur = start.elapsed();
    let per = dur.as_secs_f64() / f64::from(iters);
    println!(
        "{label:<34} per-frame {:>9.3} us   max {:>8.0} Hz   (sink {})",
        per * 1e6,
        1.0 / per,
        sink % 7
    );
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "examples/domers.toml".to_string());
    let toml = fs::read_to_string(&path).expect("read config");
    let mut config = DomersConfig::from_toml_str(&toml).expect("parse config");
    config.dome.simulation_enabled = true;
    config.bar.simulation_enabled = true;
    config.stage.simulation_enabled = true;

    // Report frame sizes for context.
    let probe = ServerState::new(config.clone());
    let frame = probe.operator_frame();
    println!(
        "frame command counts: dome={} bar={} stage={}",
        frame.dome.len(),
        frame.bar.len(),
        frame.stage.len()
    );
    println!();

    let iters = 100_000u32;

    // Full operator frame (dome + bar + stage) per selectable dome visualizer.
    for vis in 0u8..=8 {
        let mut c = config.clone();
        c.dome.active_visualizer = vis;
        let state = ServerState::new(c);
        bench(&format!("operator_frame dome_vis={vis}"), iters, || {
            let f = state.operator_frame();
            f.dome.len() + f.bar.len() + f.stage.len()
        });
    }
}
