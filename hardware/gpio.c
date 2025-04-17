#include "gpio.h"
#include "sys/mman.h"

#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Global Variables for memory mapping and I/O
int mem_fd;
void *gpio_map;

// I/O access
volatile unsigned *gpio;

// peripheral address for gpio
unsigned int current_peri_base = 0;

// Switch between BCM2710 & BCM2708 addresses
// 0 - bcm2708 = 0x20000000
// 1 - bcm2709 = 0x3f000000
// 2 - bcm2710 = 0x3f000000
// 3 - bcm2711 = 0xfe000000
int switch_hardware_address(int option) {

    if (option < 0 || option > 3) {
        printf("error: option must be 0-3\n");
        return -1;
    }

    if (gpio_map != NULL) {
        printf("found a memory map, terminating before switching...\n");
        if (terminate_gpio() != 0) {
            printf("error: failed to terminate gpio when switching hardware\n");
            return -1;
        }
    }

    if (option == 0) {
        printf("switching to bcm2708 0x%x\n", BCM2708_PERI_BASE);
        current_peri_base = BCM2708_PERI_BASE;
    } else if (option == 1) {
        printf("switching to bcm2709 0x%x\n", BCM2709_PERI_BASE);
        current_peri_base = BCM2709_PERI_BASE;
    } else if (option == 2) {
        printf("switching to bcm2710 0x%x\n", BCM2710_PERI_BASE);
        current_peri_base = BCM2710_PERI_BASE;
    } else if (option == 3) {
        printf("switching to bcm2711 0x%x\n", BCM2711_PERI_BASE);
        current_peri_base = BCM2711_PERI_BASE;
    }

    if (gpio == NULL) {
        return 0;
    }

    printf("remapping gpio with new addy\n");
    return setup_gpio();
}

int detect_peripheral_base() {
    FILE *fd;
    char buf[256];
    unsigned int gpio_base = 0, gpio_top = 0;

    // essentially cat /proc/iomem | grep gpio
    if ((fd = fopen("/proc/iomem", "r")) != NULL) {
        while (!feof(fd)) {
            fgets(buf, sizeof(buf), fd);

            if (strstr(buf, "gpio@") != NULL) {
                sscanf(buf, "%x-%x", &gpio_base, &gpio_top);
                if (gpio_base != 0) {
                    current_peri_base = gpio_base - 0x200000;
                    printf("found peripheral base: 0x%08x at GPIO at "
                           "0x%08x\n",
                           current_peri_base, gpio_base);
                    break;
                }
            }
        }
        fclose(fd);
    }

    // honestly shouldn't happen but if it does should use a known
    // address or just return -1?
    printf("current peri base: 0x%x\n", current_peri_base);
    if (current_peri_base == 0) {
        return -1;
    }

    return current_peri_base;
}

int validate_gpio_pin(int pin) {
    if (pin < GPIO_MIN_PIN || pin > GPIO_MAX_PIN) {
        printf("error: invalid gpio pin: %d (0-27)", pin);
        return -1;
    }

    return 0;
}

// Sets the GPIO pin to Input
// Validates the pin number and clears bits
int set_gpio_inp(int gpio_pin) {

    if (validate_gpio_pin(gpio_pin) < 0) {
        printf("error: invalid gpio pin; between 0-27");
        return -1;
    }

    // Clear 3 bits to the GPIO pin
    *(gpio + ((gpio_pin) / 10)) &= ~(7 << ((gpio_pin) % 10) * 3);

    return 0;
}

// Sets GPIO pin to Output
// Makes sure the pin is in input mode before output configuration
int set_gpio_out(int gpio_pin) {

    if (validate_gpio_pin(gpio_pin) < 0) {
        printf("error: invalid gpio pin; between 0-27");
        return -1;
    }

    if (set_gpio_inp(gpio_pin) != 0) {
        printf("error: failed to set_gpio_inp before set_gpio_out\n");
        return -1;
    }

    // Set output mode by changing the required bits
    *(gpio + ((gpio_pin) / 10)) |= (1 << (((gpio_pin) % 10) * 3));

    return 0;
}

// Clears the GPIO pin
// Sets it to low
int clear_gpio(int gpio_pin) {

    if (validate_gpio_pin(gpio_pin) < 0) {
        printf("error: invalid gpio pin; between 0-27");
        return -1;
    }

    // Write it to GPIO clear register for pin low
    *(gpio + GPIO_CLR_OFFSET) = 1 << gpio_pin;
    return 0;
}

// Toggles the GPIO pin; 0 - off, 1 - on
int toggle_gpio(int level, int gpio_pin) {
    if (level < 0 || level > 1) {
        printf("error: invalid level; use 0(off), or 1(on)");
        return -1;
    }

    if (validate_gpio_pin(gpio_pin) < 0) {
        printf("error: invalid gpio pin; between 0-27");
        return -1;
    }

    // Write depending on the level
    if (level == 1) {
        *(gpio + GPIO_SET_OFFSET) = 1 << gpio_pin;
    } else if (level == 0) {
        *(gpio + GPIO_CLR_OFFSET) = 1 << gpio_pin;
    }

    return 0;
}

// Read and get status of a GPIO pin
int get_gpio(int gpio_pin) {

    if (validate_gpio_pin(gpio_pin) < 0) {
        printf("error: invalid gpio pin; between 0-27");
        return -1;
    }

    // Read from GPIO level register and
    // return the bit for the pin
    return ((*(gpio + GPIO_LEV_OFFSET) & (1 << gpio_pin)) ? 1 : 0);
}

// Set up a memory regions to access GPIO
int setup_gpio() {
    printf("gpio: setting up with address 0x%x\n", current_peri_base);

    /* open /dev/mem */
    if ((mem_fd = open("/dev/mem", O_RDWR | O_SYNC)) < 0) {
        printf("error: can't open /dev/mem \n");
        return -1;
    }

    unsigned int gpio_base = current_peri_base + GPIO_HW_OFFSET;

    /* mmap GPIO */
    gpio_map = mmap(
        NULL,                   // Any adddress in our space will do
        BLOCK_SIZE,             // Map length
        PROT_READ | PROT_WRITE, // Enable reading & writting to mapped memory
        MAP_SHARED,             // Shared with other processes
        mem_fd,                 // File to map
        gpio_base               // Offset to GPIO peripheral
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

// Set up pull-down resistor for a GPIO pin
// wait_time in useconds used for delaying
int set_gpio_pulldown(int gpio_pin, int wait_time) {

    if (wait_time < 0) {
        wait_time = 100;
    }

    if (validate_gpio_pin(gpio_pin) < 0) {
        printf("error: invalid gpio pin; between 0-27");
        return -1;
    }

    // clear first
    *(gpio + GPIO_PULL_OFFSET) = 0;
    usleep(wait_time);

    *(gpio + GPIO_PULL_OFFSET) = 1;
    usleep(wait_time);

    // clock it
    *(gpio + GPIO_PULLCLK0_OFFSET) = (1 << gpio_pin);
    usleep(wait_time);

    // clear em
    *(gpio + GPIO_PULL_OFFSET) = 0;
    *(gpio + GPIO_PULLCLK0_OFFSET) = 0;

    return 0;
}

// Set up pull-up resistor for a GPIO pin
// wait_time in useconds
int set_gpio_pullup(int gpio_pin, int wait_time) {

    if (wait_time < 0) {
        wait_time = 100;
    }

    if (validate_gpio_pin(gpio_pin) < 0) {
        printf("error: invalid gpio pin; between 0-27");
        return -1;
    }

    // clear first
    *(gpio + GPIO_PULL_OFFSET) = 0;
    usleep(wait_time);

    *(gpio + GPIO_PULL_OFFSET) = 2;
    usleep(wait_time);

    // clock it
    *(gpio + GPIO_PULLCLK0_OFFSET) = (1 << gpio_pin);
    usleep(wait_time);

    // clear em
    *(gpio + GPIO_PULL_OFFSET) = 0;
    *(gpio + GPIO_PULLCLK0_OFFSET) = 0;

    return 0;
}

// Handle Clean Up
// Release the memory-mapped GPIO
int terminate_gpio() {
    printf("gpio: cleaning up\n");
    // Unmap the GPIO memory if it is mapped
    if (gpio_map != NULL) {
        if (munmap(gpio_map, BLOCK_SIZE) == 0) {
            gpio_map = NULL;
        } else {
            return -1;
        }
    }
    return 0;
}
