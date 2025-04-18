# pipin.rs  
[![Release](https://img.shields.io/github/v/release/nuttycream/pipin.rs)](https://github.com/nuttycream/pipin.rs/releases)  
<p align="center"> 
  <img src="https://github.com/user-attachments/assets/f9537cfe-b7ee-4dc9-8c86-3f0a184a50bd" width="50%">
</p>  

A simple self contained application to control gpio pins from your browser

## Features

- Toggle individual pins (0-27)
- Wicked fast toggling through WebSockets
- Queue various actions
  - Toggle
  - Delay(ms)
  - Wait For High
  - Wait For Low
  - Pull Down
  - Pull Up
- Loop action sequences
- Small self-contained executable (approx ~1MB)

## Installation

- Download from latest
  [release](https://github.com/nuttycream/pipin.rs/releases)
- Extract to Raspberry Pi

```sh
# Note: if using ssh, you can download on your main machine and rysnc/scp the executable
# onto the pi. Example:
# rsync
rsync -avz pipinrs user@hostname:~/pipin
# scp
scp pipinrs user@pi-host:~/pipin
```

## Usage

- Run the program:

```sh
cd <directory>
sudo ./pipinrs # sudo is needed for direct registry access
```

- By default it uses port 3000, to use a different port:

```sh
sudo ./pipinrs 8080 #runs on port 8080
```

- Navigate to webpage; defaults to `0.0.0.0:3000` or `localhost:3000`

```
Note: you can also navigate on any machine connected to the same network,
by using the raspberry pi's IP address and port number,
example: http://192.168.68.70:3000
```

- Press 'Initialize' to setup the GPIO pins
- bobs ur uncle

## To-Do

- [ ] Device address override
- [ ] Pins should show whether they're high or low
- [ ] Individual GPIO pin pulldown/up

## Attribution

gpio.h and gpio.c library was modified from
[gpio direct registry access example](https://elinux.org/RPi_GPIO_Code_Samples#Direct_register_access)
