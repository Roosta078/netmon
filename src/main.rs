// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{error::Error, sync::{Arc, Mutex}, thread, time::Duration, fs};
use slint::{Timer, TimerMode};
use serde::Deserialize;
use rumqttc::{Client, Event, MqttOptions, Packet, QoS};
use rumqttc;
use log::{info, warn, error, debug};
slint::include_modules!();

#[derive(Deserialize, Debug)]
struct MQTTConfig {
    host: String,
    port: u16,
    topic: String,
    id: String,
    cap: usize
}


#[derive(Deserialize, Debug)]
struct Config {
    mqtt_broker: MQTTConfig,
}

struct NetStats{
    rx:f64,
    tx:f64,
}


fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let config_file = "config.toml";
    let config: Config = toml::from_str(&fs::read_to_string(config_file)?)?;
    info!("Got config to connect to mqtt broker at {}:{}", config.mqtt_broker.host, config.mqtt_broker.port);
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
    let handle: thread::JoinHandle<()> = thread::spawn(move || {
        let mut myqttopts = MqttOptions::new(config.mqtt_broker.id, config.mqtt_broker.host, config.mqtt_broker.port);
        myqttopts.set_keep_alive(Duration::from_secs(5));
        let (client, mut connection) = Client::new(myqttopts, config.mqtt_broker.cap);
        client.subscribe(config.mqtt_broker.topic, QoS::AtMostOnce).expect("subcribe failed");
        for event in connection.iter() {
            debug!("In event loop");
            if !(*run_cln.lock().unwrap()) {
                break;
            }
            match event {
                Ok(Event::Incoming(Packet::ConnAck(p))) => {
                        if p.code==rumqttc::ConnectReturnCode::Success {
                            info!("Connection Successful")
                        } else {
                            warn!("Connection failed with code: {:?}", p.code)
                        }
                    },
                Ok(Event::Incoming(Packet::SubAck(p))) => {
                        if p.return_codes.contains(&rumqttc::SubscribeReasonCode::Failure) {
                            warn!("Subscribe Failed");
                        } else {
                            info!("Subscribe Success")
                        }
                    },
                Ok(Event::Incoming(Packet::Publish(p))) => {
                        let mut payload = String::from_utf8_lossy(&p.payload).into_owned();
                        payload.retain(|c| c != '\0');
                        let payload = payload.trim();
                        debug!("Got payload: {}", payload);
                        let parts: Vec<&str> = payload.split(':').collect();
                        if parts.len() > 2 {
                            let rx = parts[1].trim().parse::<f64>().unwrap_or(0.0);
                            let tx = parts[2].trim().parse::<f64>().unwrap_or(0.0);
                            let mut stats = thr_shared.lock().unwrap();
                            stats.rx = rx;
                            stats.tx = tx;
                        } else {
                            warn!("Payload likely malformed: {}", payload);
                        }
                    },
                Ok(Event::Incoming(p)) => debug!("incoming {:?}", p),
                Err(e)=> error!("MQTT Client Recieved Error{:#?}", e),
                Ok(Event::Outgoing(p)) => debug!("outgoing {:?}", p),
            };
            
        }
    });
    
    ui.run()?;
    *running.lock().unwrap() = false;
    handle.join().unwrap();
    Ok(())
}