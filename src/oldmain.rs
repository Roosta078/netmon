// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{error::Error, sync::{Arc, Mutex}, thread, time::Duration};
use slint::{Timer, TimerMode};
use rand::Rng;

slint::include_modules!();

struct NetStats{
    rx:f64,
    tx:f64,
}

fn get_speed()->NetStats{
    let mut rng = rand::rng();
    let rx = rng.random::<f64>()*100.0;
    let tx = rng.random::<f64>()*100.0;
    NetStats {rx, tx}
}


fn main() -> Result<(), Box<dyn Error>> {
    let ui:AppWindow = AppWindow::new()?;
    let weak_handle = ui.as_weak();
    let timer = Timer::default();
    

    let out_speed: Arc<Mutex<NetStats>> = Arc::new(Mutex::new(NetStats { rx: 0.0, tx: 0.0 }));
    let tim_shared = Arc::clone(&out_speed);
    let running = Arc::new(Mutex::new(true));


    timer.start(TimerMode::Repeated, std::time::Duration::from_millis(200), 
        move || {
            if let Some(window) = weak_handle.upgrade(){
                let current_stats = tim_shared.lock().unwrap();
                let in_str = format!("{:.2} Mbps ↑",current_stats.tx);
                window.set_net_in(in_str.into());

                let out_str = format!("{:.2} Mbps ↓",current_stats.rx);
                window.set_net_out(out_str.into());
            }
    });

    let thr_shared: Arc<Mutex<NetStats>> = Arc::clone(&out_speed);
    let run_cln: Arc<Mutex<bool>> = Arc::clone(&running);
    let handle = thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(200));
            if *run_cln.lock().unwrap() == false {
                break;
            }
            *thr_shared.lock().unwrap() = get_speed();
        }
        println!("exiting thread");

    });
    
    ui.run()?;
    *running.lock().unwrap() = false;
    handle.join().unwrap();
    Ok(())
}