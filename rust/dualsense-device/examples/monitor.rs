//! Hardware smoke test: read the pad for a few seconds (no virtual pad) and print what the service sees.
//! Run: cargo run -p dualsense-device --example monitor

use dualsense_core::profile::MapProfile;
use dualsense_device::service::EmulatorService;
use std::time::Duration;

fn main() {
    let mut service = EmulatorService::new(MapProfile::load(&MapProfile::default_path()), |s| {
        println!("status: {:?} - {}", s.state, s.message)
    });
    service.start();
    service.stop(); // read only; emulation stays off
    for _ in 0..10 {
        std::thread::sleep(Duration::from_millis(500));
        match service.latest() {
            Some(s) => println!(
                "{:?} battery={} charge={:?} lx={} buttons={:#x}",
                service.transport(),
                s.ds.battery_percent,
                s.ds.charge,
                s.ds.lx,
                s.ds.buttons
            ),
            None => println!("no input yet ({:?})", service.transport()),
        }
    }
}
