# pipinctrl.rs
control gpio pins using your browser 

`Note: currently supports Raspberry Pi GPIO Addresses`

## features
- toggle individual pins (0-27)
- queue up different pins to toggle
- uwuify

## installation
- download from latest release
- extract to raspberry pi
```sh
# Note: if using ssh, you can download on your main machine and rysnc/scp the executable
# onto the pi. Example:
# rsync
rsync -avz ''
# scp
scp user@host:<insert downloaded file here>:<raspberry pi folder>
```

## usage
- run the program:
```sh
cd <directory>
sudo ./pipinctrlrs # sudo is needed for direct registry access
```
- navigate to webpage; defaults to `0.0.0.0:3000` or `localhost:3000`

```
Note: you can also navigate on any machine connected to the same network,
by using the raspberry pi's IP address and port number,
example: http://192.168.68.70:3000
```

- select chipset (defaults to BCM2710)
- press 'Initialize'
- bobs ur uncle

## attribution
gpio.h and gpio.c library was been modified from [gpio direct registry access example](https://elinux.org/RPi_GPIO_Code_Samples#Direct_register_access) 







