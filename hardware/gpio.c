#include "gpio.h"
#include "sys/mman.h"
#include <stdio.h>
#include <stdlib.h>

int mem_fd;
void *gpio_map;

// I/O access
volatile unsigned *gpio;

// switch between bcm2710 & bcm2708 addresses
// defaults to bcm2710
// 0 - bcm2710 = 0x3f000000
// 1 - bcm2708 = 0x20000000
extern int switch_hardware_address(int option) {

    if (option < 0 || option > 1) {
        printf("error: option must be 0-1\n");
        return -1;
    }

    return 0;
}

// Sets GPIO pin to Input
extern int set_gpio_inp(int gpio_pin) {

    if (gpio_pin < 0 || gpio_pin > 27) {
        printf("error: invalid gpio pin; between 0-27");
        return -1;
    }

    *(gpio + ((gpio_pin) / 10)) &= ~(7 << ((gpio_pin) % 10) * 3);

    return 0;
}

// Sets GPIO pin to Output
extern int set_gpio_out(int gpio_pin) {

    if (gpio_pin < 0 || gpio_pin > 27) {
        printf("error: invalid gpio pin; between 0-27");
        return -1;
    }

    if (set_gpio_inp(gpio_pin) != 0) {
        printf("error: failed to set_gpio_inp before set_gpio_out\n");
        return -1;
    }

    *(gpio + ((gpio_pin) / 10)) |= (1 << (((gpio_pin) % 10) * 3));

    return 0;
}

// Clear gpio
extern int clear_gpio(int gpio_pin) {

    if (gpio_pin < 0 || gpio_pin > 27) {
        printf("error: invalid gpio pin; between 0-27");
        return -1;
    }

    *(gpio + 10) = 1 << gpio_pin;
    return 0;
}

// Toggles the GPIO pin; 0 - off, 1 - on
extern int toggle_gpio(int level, int gpio_pin) {
    if (level < 0 || level > 1) {
        printf("error: invalid level; use 0(off), or 1(on)");
        return -1;
    }

    if (gpio_pin < 0 || gpio_pin > 27) {
        printf("error: invalid gpio pin; between 0-27");
        return -1;
    }

    if (level == 1) {
        *(gpio + 7) = 1 << gpio_pin;
    } else if (level == 0) {
        *(gpio + 10) = 1 << gpio_pin;
    }

    return 0;
}

// Get GPIO status
extern int get_gpio(int gpio_pin) {

    if (gpio_pin < 0 || gpio_pin > 27) {
        printf("error: invalid gpio pin; between 0-27");
        return -1;
    }

    return ((*(gpio + 13) & (1 << gpio_pin)) ? 1 : 0);
}

// Set up a memory regions to access GPIO
extern int setup_io() {
    printf("gpio: setting up\n");
    /* open /dev/mem */
    if ((mem_fd = open("/dev/mem", O_RDWR | O_SYNC)) < 0) {
        printf("error: can't open /dev/mem \n");
        return -1;
    }

    /* mmap GPIO */
    gpio_map = mmap(
        NULL,                   // Any adddress in our space will do
        BLOCK_SIZE,             // Map length
        PROT_READ | PROT_WRITE, // Enable reading & writting to mapped memory
        MAP_SHARED,             // Shared with other processes
        mem_fd,                 // File to map
        GPIO_BASE               // Offset to GPIO peripheral
    );

    close(mem_fd); // No need to keep mem_fd open after mmap

    if (gpio_map == MAP_FAILED) {
        printf("mmap error %p\n", gpio_map); // errno also set!
        exit(-1);
    }

    // Always use volatile pointer!
    gpio = (volatile unsigned *)gpio_map;

    return 0;
}

// Set up pull-down resistor for a gpio pin
// wait_time in useconds
extern int set_gpio_pulldown(int gpio_pin, int wait_time) {

    if (gpio_pin < 0 || gpio_pin > 27) {
        printf("error: invalid gpio pin; between 0-27");
        return -1;
    }

    // clear first
    GPIO_PULL = 0;
    usleep(wait_time);

    GPIO_PULL = 1;
    usleep(wait_time);

    // clock it
    GPIO_PULLCLK0 = (1 << gpio_pin);
    usleep(wait_time);

    // clear em
    GPIO_PULL = 0;
    GPIO_PULLCLK0 = 0;

    return 0;
}

// Set up pull-up resistor for gpion pin
// wait_time in useconds
extern int set_gpio_pullup(int gpio_pin, int wait_time) {

    if (gpio_pin < 0 || gpio_pin > 27) {
        printf("error: invalid gpio pin; between 0-27");
        return -1;
    }

    GPIO_PULL = 0;
    usleep(wait_time);

    GPIO_PULL = 2;
    usleep(wait_time);

    // clock it
    GPIO_PULLCLK0 = (1 << gpio_pin);
    usleep(wait_time);

    // clear em
    GPIO_PULL = 0;
    GPIO_PULLCLK0 = 0;

    return 0;
}

// Handle Clean up
extern int terminate_io() {
    printf("gpio: cleaning up\n");
    // Unmap the GPIO memory
    if (gpio_map != NULL) {
        if (munmap(gpio_map, BLOCK_SIZE) == 0) {
            gpio_map = NULL;
        } else {
            return -1;
        }
    }
    return 0;
}
