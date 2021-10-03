# Sherline Power Feed

Rust firmware for an STM32 controlled stepper motor, providing power feed for the Sherline lathe.

## Components

* STM32F103C8T6 MCU (BluePill package)
* 3 position rotary switch
  * direction/off
* momentary push button
  * enabled rapid feed
* NC microswitch
  * limit switch for crash avoidance
* rotary encoder
  * feed rate control
* NEMA 23 3Nm stepper
* DM543T stepper driver
* 36Vdc power supply

## Wiring

**STM32 - Peripheral**

* Rotary Switch
  * A8  - Pos 1+
  * A9  - Pos 2+
  * GND - Pos 1-
  * GND - Pos 2-
* Rapid switch
  * A10 - Sw+
  * GND - Sw-
* Limit Switch
  * A11 - Sw+
  * GND - Sw-
* Rotary Encoder
  * B6  - A (with parallel 10k resistor to 5v)
  * GND - GND
  * B7  - B (with parallel 10k resistor to 5v)
* OLED Display
  * 5v  - VCC
  * GND - GND
  * B8  - SCL
  * B9  - SDA
* Stepper Driver
  * A0  - Pulse+
  * A1  - Dir+
  * GND - Pulse-
  * GND - Dir-


