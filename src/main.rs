// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{error::Error, sync::{Arc, Mutex}, thread, time::Duration};
use slint::{Timer, TimerMode};

use rumqttc::{Client, Event, MqttOptions, Packet, QoS};

slint::include_modules!();

struct NetStats{
    rx:f64,
    tx:f64,
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
                let in_str = format!("{:.2} Mbps ↑",current_stats.tx * 8.0 / 1_000_000.0);
                window.set_net_in(in_str.into());

                let out_str = format!("{:.2} Mbps ↓",current_stats.rx * 8.0 / 1_000_000.0);
                window.set_net_out(out_str.into());
            }
    });

    let thr_shared: Arc<Mutex<NetStats>> = Arc::clone(&out_speed);
    let run_cln: Arc<Mutex<bool>> = Arc::clone(&running);
    let handle = thread::spawn(move || {
        let mut myqttopts = MqttOptions::new("netmon", "192.168.10.10", 1883);
        myqttopts.set_keep_alive(Duration::from_secs(30));
        let (client, mut connection) = Client::new(myqttopts, 10);
        client.subscribe("collectd/OpenWrt/interface-eth1/if_octets", QoS::AtMostOnce).expect("subcribe failed");
        
        for event in connection.iter() {
            if !(*run_cln.lock().unwrap()) {
                break;
            }
            if let Ok(Event::Incoming(Packet::Publish(p))) = event  {
                let mut payload = String::from_utf8_lossy(&p.payload).into_owned();
                payload.retain(|c| c != '\0');
                let payload = payload.trim();
                
                let parts: Vec<&str> = payload.split(':').collect();
                if parts.len() > 2 {
                    let rx = parts[1].trim().parse::<f64>().unwrap_or(0.0);
                    let tx = parts[2].trim().parse::<f64>().unwrap_or(0.0);
                    let mut stats = thr_shared.lock().unwrap();
                    stats.rx = rx;
                    stats.tx = tx;
                }
            }
        }
    });
    
    ui.run()?;
    *running.lock().unwrap() = false;
    handle.join().unwrap();
    Ok(())
}