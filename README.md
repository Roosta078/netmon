# netmon

A network monitoring GUI built in rust.
This is currently developed for Linux, with future plans to run on an embedded device.

## How it works
The program gets network data over MQTT.  This means you need an MQTT broker and something to publish network data.  The former is outside the scope of this document.  The later is achieved, in my case, with collectd running on an OpenWRT router.  Your specific MQTT information should be added to `config.toml`.
