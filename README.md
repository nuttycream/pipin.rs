# pipin.rs

A simple self contained application to control gpio pins from your browser

## features

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

## installation

- download from latest release
- extract to raspberry pi

```sh
# Note: if using ssh, you can download on your main machine and rysnc/scp the executable
# onto the pi. Example:
# rsync
rsync -avz pipinrs user@hostname:~/pipin
# scp
scp pipinrs user@pi-host:~/pipin
```

## usage

- Run the program:

```sh
cd <directory>
sudo ./pipinctrlrs # sudo is needed for direct registry access
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

## attribution

gpio.h and gpio.c library was modified from
[gpio direct registry access example](https://elinux.org/RPi_GPIO_Code_Samples#Direct_register_access)
