# pipin

![GitHub Release](https://img.shields.io/github/v/release/nuttycream/pipin?label=Release)
[![Release](https://github.com/nuttycream/pipin/actions/workflows/release.yml/badge.svg)](https://github.com/nuttycream/pipin/actions/workflows/release.yml)
![Download Size](https://img.shields.io/badge/Download%20Size-539%20KB-blue)

<p align="center">
    A simple self contained application to control gpio pins from your browser
    <img src="https://i.imgur.com/aZgyDpJ.png">
</p>

## Features

- Toggle individual GPIO Pins (0-27)
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

- Download from latest [release](https://github.com/nuttycream/pipin/releases)
- Extract to Raspberry Pi

```sh
# Note: You can download on your main machine and rysnc/scp the executable
# onto the pi. Example:
# rsync
rsync -avz pipin user@hostname:~/pipin
# scp
scp pipin user@pi-host:~/pipin
```

## Usage

- Run the program:

```sh
cd <directory>
./pipin
```

- By default it uses port 3000, to use a different port:

```sh
sudo ./pipin 8080 #runs on port 8080
```

- Navigate to webpage; defaults to `0.0.0.0:3000` or `localhost:3000`

```
Note: you can also navigate on any machine connected to the same network,
by using the raspberry pi's IP address and port number,
example: http://192.168.68.70:3000
```

- Press 'Setup' to initialize the GPIO pins
- bobs ur uncle

## Similar

- [pigg](https://github.com/andrewdavidmackenzie/pigg) - GUI for remote control
  of Raspberry Pi GPIO hardware

## Attribution

[GPIO Direct Registry Access in C](https://elinux.org/RPi_GPIO_Code_Samples#Direct_register_access)
