
# thinkfan-tui

A terminal-based Linux application for fan control and temperature monitoring on ThinkPad laptops.

![Screenshot](screenshot.gif "Screenshot")

# How it Works?

The application continously runs the `sensors` command to read temperatures and display these in the terminal. To control the fan speed, commands are written to the `/proc/acpi/ibm/fan` file. If the user lacks permissions to do so the owner of the file is changed to the current user by calling the `pkexec` command.

# Keyboard Shortcuts

| Key  | Action                          |
| ---- | ------------------------------- |
| 0..7 | Set fan speed to specific level |
| A    | Set fan speed to automatic      |
| F    | Set fan speed to full           |
| Q    | Quit application                |

# Dependencies

The project uses `lm-sensors` and `policykit`. These can be installed with the commands below.

## Ubuntu

> sudo apt install lm-sensors policykit-1

## Arch Linux

> sudo pacman -S lm_sensors polkit

# Running

## Pre-built

The easiest way is to download the latest binary from https://github.com/karjonas/thinkfan-tui/releases, unzip it, make it executable and run.

## Building

This project is written in the rust programming language and is built using cargo, see https://www.rust-lang.org/tools/install for installation instructions.

To build and run `thinkfan-tui`, checkout the repository and run:

> cargo run --release

# Contributing

Please report any issues you find at https://github.com/karjonas/thinkfan-tui. Outputs from the `sensors -j` command are also appreciated for more test coverage on different laptops.

# License

Distributed under the MIT License. See LICENSE for more information.

# Contact

This project is hosted at https://github.com/karjonas/thinkfan-tui

# Acknowledgements

`thinkfan-tui` is inspired by [Thinkfan UI](https://github.com/zocker-160/thinkfan-ui).