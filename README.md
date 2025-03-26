<h2 align="center">
    pipinctrl.rs
</h2>

<!--toc:start-->
- [features](#features)
- [installation](#installation)
- [usage](#usage)
- [attribution](#attribution)
<!--toc:end-->
control gpio pins using your browser

## features
- toggle individual pins (0-27)
- queue up different pins to toggle
- uwuify

## installation
- download from latest release
- extract to raspberry pi
`Note: if using ssh, you can download on your main machine and rysnc/scp the executable
onto the pi. Example:`
```sh
# rsync
rsync -avz ''
# scp
scp user@host:<insert downloaded file here>:<raspberry pi folder>
```

## usage
- select chipset (defaults to BCM2710)
- press 'Initialize'
- bobs ur uncle

## attribution
modified from: [this example](https://elinux.org/RPi_GPIO_Code_Samples#Direct_register_access) 







