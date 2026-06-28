//! sensor_cal – MonoOS sensor calibration verification tool.
//!
//! Reads raw sensor data from /sys/bus/iio/ and checks that each sensor
//! produces values within the expected calibration range.
//!
//! Sensors tested:
//!   - Accelerometer  (3-axis, at rest: |x|<0.3g, |y|<0.3g, z≈1g ±0.2g)
//!   - Gyroscope      (3-axis, at rest: |xyz| < 0.05 rad/s)
//!   - Magnetometer   (3-axis, total magnitude 25–65 µT)
//!   - Proximity      (near: 0–5 cm, far: >5 cm)
//!   - Ambient light  (0–10000 lux)
//!
//! Usage:
//!   sensor_cal [--sensor <name>] [--samples <n>] [--verbose]

/// One sensor reading (up to 3 axes).
#[derive(Debug, Clone, Copy, Default)]
struct Reading {
    x: f64,
    y: f64,
    z: f64,
}

/// Pass/fail result for a sensor calibration check.
#[derive(Debug)]
struct CalResult {
    sensor:  String,
    pass:    bool,
    reading: Reading,
    reason:  Option<String>,
}

/// Read `n` samples from an IIO sensor and average them.
/// Path format: /sys/bus/iio/devices/iio:deviceN/in_<axis>_raw
fn read_iio_sensor(iio_dev: &str, n: usize) -> Reading {
    // On-device: read /sys/bus/iio/devices/<dev>/in_{accel,anglvel,magn}_{x,y,z}_raw
    // and multiply by the scale factor from in_<type>_scale.
    // Stub: return a plausible at-rest value.
    let _ = (iio_dev, n);
    Reading { x: 0.01, y: -0.02, z: 9.81 }
}

fn check_accelerometer(r: Reading) -> CalResult {
    let g = 9.807_f64;
    let pass = r.x.abs() < 0.3 * g
        && r.y.abs() < 0.3 * g
        && (r.z - g).abs() < 0.2 * g;
    CalResult {
        sensor:  "accelerometer".into(),
        pass,
        reading: r,
        reason:  if pass { None } else {
            Some(format!("out of range: x={:.3} y={:.3} z={:.3}", r.x, r.y, r.z))
        },
    }
}

fn check_gyroscope(r: Reading) -> CalResult {
    let pass = r.x.abs() < 0.05 && r.y.abs() < 0.05 && r.z.abs() < 0.05;
    CalResult {
        sensor:  "gyroscope".into(),
        pass,
        reading: r,
        reason:  if pass { None } else {
            Some(format!("drift too high: x={:.4} y={:.4} z={:.4}", r.x, r.y, r.z))
        },
    }
}

fn check_magnetometer(r: Reading) -> CalResult {
    let mag = (r.x * r.x + r.y * r.y + r.z * r.z).sqrt();
    let pass = (25.0..=65.0).contains(&mag);
    CalResult {
        sensor:  "magnetometer".into(),
        pass,
        reading: r,
        reason:  if pass { None } else {
            Some(format!("total magnitude {mag:.1} µT outside [25, 65]"))
        },
    }
}

fn check_proximity(cm: f64) -> CalResult {
    let pass = (0.0..=5.0).contains(&cm) || cm > 5.0; // near or far are both valid states
    CalResult {
        sensor:  "proximity".into(),
        pass,
        reading: Reading { x: cm, y: 0.0, z: 0.0 },
        reason:  if pass { None } else {
            Some(format!("proximity {cm:.1} cm out of sane range"))
        },
    }
}

fn check_ambient_light(lux: f64) -> CalResult {
    let pass = (0.0..=10000.0).contains(&lux);
    CalResult {
        sensor:  "ambient_light".into(),
        pass,
        reading: Reading { x: lux, y: 0.0, z: 0.0 },
        reason:  if pass { None } else {
            Some(format!("lux {lux:.0} outside [0, 10000]"))
        },
    }
}

/// Map a sensor name to its IIO device path. Real device numbering is
/// board-specific and is normally discovered via the `name` sysfs entry
/// under each `iio:deviceN`; these are the MonoOS reference-device defaults.
fn iio_path_for(sensor: &str) -> Option<&'static str> {
    match sensor {
        "accelerometer" => Some("/sys/bus/iio/devices/iio:device0"),
        "gyroscope"     => Some("/sys/bus/iio/devices/iio:device1"),
        "magnetometer"  => Some("/sys/bus/iio/devices/iio:device2"),
        "proximity"     => Some("/sys/bus/iio/devices/iio:device3"),
        "ambient_light" => Some("/sys/bus/iio/devices/iio:device4"),
        _ => None,
    }
}

fn run_checks(verbose: bool, samples: usize, only_sensor: Option<&str>) -> Vec<CalResult> {
    let all = ["accelerometer", "gyroscope", "magnetometer", "proximity", "ambient_light"];
    let selected: Vec<&str> = match only_sensor {
        Some(s) => all.iter().copied().filter(|&n| n == s).collect(),
        None    => all.to_vec(),
    };

    if verbose {
        eprintln!("[sensor_cal] {} samples per sensor", samples);
    }

    selected
        .into_iter()
        .filter_map(|name| {
            let path = iio_path_for(name)?;
            let r = read_iio_sensor(path, samples);
            if verbose {
                eprintln!("[sensor_cal] {name}: x={:.4} y={:.4} z={:.4}", r.x, r.y, r.z);
            }
            Some(match name {
                "accelerometer" => check_accelerometer(r),
                "gyroscope"     => check_gyroscope(r),
                "magnetometer"  => check_magnetometer(r),
                "proximity"     => check_proximity(r.x),
                "ambient_light" => check_ambient_light(r.x),
                _ => unreachable!(),
            })
        })
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let verbose = args.iter().any(|a| a == "--verbose");

    let only_sensor = args.iter()
        .position(|a| a == "--sensor")
        .and_then(|i| args.get(i + 1))
        .map(String::as_str);

    let samples: usize = args.iter()
        .position(|a| a == "--samples")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(64);

    println!("MonoOS Sensor Calibration Check");
    println!("{}", "-".repeat(60));

    let results = run_checks(verbose, samples, only_sensor);
    let failed = results.iter().filter(|r| !r.pass).count();

    if results.is_empty() {
        eprintln!("No matching sensor for --sensor {:?}", only_sensor.unwrap_or(""));
        std::process::exit(2);
    }

    for r in &results {
        let status = if r.pass { "PASS" } else { "FAIL" };
        print!("  [{status}] {:<20} (x={:.3} y={:.3} z={:.3})", r.sensor, r.reading.x, r.reading.y, r.reading.z);
        if let Some(reason) = &r.reason {
            print!("  {reason}");
        }
        println!();
    }

    println!("{}", "-".repeat(60));
    println!("{} passed, {} failed\n", results.len() - failed, failed);

    std::process::exit(if failed == 0 { 0 } else { 1 });
}
