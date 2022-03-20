# Amateur Radio Station Remote Closedown Controller [![CI](https://github.com/DanNixon/remote-closedown/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/DanNixon/remote-closedown/actions/workflows/ci.yml)

Simple tool used to kill transmission from a remote amateur radio station, gateway or repeater.

Or a very over-engineered means of complying with licence terms for remote stations, gateways and repeaters.

## Features

- Allows controlling the following via MQTT:
  - TX/radio/PA power
  - PTT/transmit enable
- Provides feedback of the following via MQTT:
  - TX/radio/PA power state
  - Transmit state
- Can automatically (attempt to) disable transmission if the radio has been transmitting for too long (e.g. PTT becoming latched for whatever reason)

## Configuration

See [the example](./examples/config.toml).

`tx_guard_time` is specified in milliseconds.
IO pin numbers are expected as per their appearance in sysfs (for the Raspberry Pi, this means the "Broadcom"/"chip" numbering).

## Usage

See `remote-closedown --help`.
