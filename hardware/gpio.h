#include <stdio.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <unistd.h>

// Addresses for BCM and GPIO
#define BCM2708_PERI_BASE 0x20000000
#define BCM2709_PERI_BASE 0x3f000000
#define BCM2710_PERI_BASE 0x3f000000
#define BCM2711_PERI_BASE 0xfe000000
#define GPIO_HW_OFFSET 0x200000
extern unsigned int current_peri_base;

// Memory Mapping
#define PAGE_SIZE (4 * 1024)
#define BLOCK_SIZE (4 * 1024)

// gpio register offsets
#define GPIO_SET_OFFSET 7
#define GPIO_CLR_OFFSET 10
#define GPIO_LEV_OFFSET 13
#define GPIO_PULL_OFFSET 37
#define GPIO_PULLCLK0_OFFSET 38

#define GPIO_MIN_PIN 0
#define GPIO_MAX_PIN 27

// I/O access
extern volatile unsigned *gpio;

// Set up a memory regions to access GPIO
extern int setup_gpio();
extern int terminate_gpio();

// Switch between BCM2710 & BCM2708 addresses
// 0 - bcm2708 = 0x20000000
// 1 - bcm2709 = 0x3f000000
// 2 - bcm2710 = 0x3f000000
// 3 - bcm2711 = 0xfe000000
extern int switch_hardware_address(int option);

// Helper func to detect base peripheral address
int detect_peripheral_base();

// validate gpio pin between 0 - 27
extern int validate_gpio_pin(int pin);

// Set GPIO direction
extern int set_gpio_inp(int gpio_pin);
extern int set_gpio_out(int gpio_pin);

// Clear GPIO
extern int clear_gpio(int gpio_pin);

// Get GPIO; 0 - low, 1 - high
extern int get_gpio(int gpio_pin);

// Toggles the GPIO pin: 0 - off, 1 - on
extern int toggle_gpio(int level, int gpio_pin);

// Set up pull-down resistor for a GPIO pin
extern int set_gpio_pulldown(int gpio_pin, int wait_time);
// Set up pull-up resistor for GPIO pin
extern int set_gpio_pullup(int gpio_pin, int wait_time);
