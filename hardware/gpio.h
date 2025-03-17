// Addresses
#define BCM2710_PERI_BASE 0x3f000000
#define BCM2708_PERI_BASE 0x20000000
#define GPIO_BASE (BCM2710_PERI_BASE + 0x200000)

#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <unistd.h>

#define PAGE_SIZE (4 * 1024)
#define BLOCK_SIZE (4 * 1024)

// I/O access
extern volatile unsigned *gpio;

// GPIO setup macros. Always use INP_GPIO(x) before using OUT_GPIO(x)
// or SET_GPIO_ALT(x,y)
#define INP_GPIO(g) *(gpio + ((g) / 10)) &= ~(7 << (((g) % 10) * 3))
#define OUT_GPIO(g) *(gpio + ((g) / 10)) |= (1 << (((g) % 10) * 3))
#define SET_GPIO_ALT(g, a)                                                     \
    *(gpio + (((g) / 10))) |= (((a) <= 3   ? (a) + 4                           \
                                : (a) == 4 ? 3                                 \
                                           : 2)                                \
                               << (((g) % 10) * 3))

#define GPIO_SET *(gpio + 7) // sets   bits which are 1 ignores bits which are 0
#define GPIO_CLR                                                               \
    *(gpio + 10) // clears bits which are 1 ignores bits which are 0

#define GET_GPIO(g) (*(gpio + 13) & (1 << g)) // 0 if LOW, (1<<g) if HIGH
#define GPIO_PULL *(gpio + 37)                // Pull up/pull down
#define GPIO_PULLCLK0 *(gpio + 38)            // Pull up/pull down clock

// Set GPIO direction
int set_gpio_inp(int gpio_pin);
int set_gpio_out(int gpio_pin);

// Clear gpio
int clear_gpio(int gpio_pin);

// get gpio
// 0 - LOW
// 1 - HIGH
int get_gpio(int gpio_pin);

// Toggles the GPIO pin: 0 - off, 1 - on
int toggle_gpio(int level, int gpio_pin);

// Set up pull-down resistor for a gpio pin
int set_gpio_pulldown(int gpio_pin, int wait_time);
// Set up pull-up resistor for gpion pin
int set_gpio_pullup(int gpio_pin, int wait_time);

// Set up a memory regions to access GPIO
int setup_io();

// Clean up
int terminate_io();
