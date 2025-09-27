# native-serial

native-serial is a small, fast native Node.js addon (built with N-API and Rust via napi-rs)
that provides cross-platform serial port discovery and a safe, single-threaded worker for
opening and communicating with serial ports.

Key features:

- List available serial ports and USB details
- Open a port and receive data via a threadsafe callback
- Write bytes to the port and receive write error callbacks
- Small, dependency-free Rust implementation using the `serialport` crate

This repository contains the Rust source (in `src/`) and the JS wrapper files
(`index.js`, `index.d.ts`) used to load the native binary.

## Installation

Prebuilt binaries may be available for common platforms. To install via npm/yarn:

```bash
yarn add native-serial
# or
npm install native-serial
```

If you're developing locally you'll need Rust and the Node toolchain (see "Build from source").

## Quick API overview

Exports (from the native addon):

- `list_ports(): Promise<Port[]>` — list available serial ports

`Port` (object) has the following fields and methods:

- `path: string` — path to device (e.g. `/dev/ttyUSB0` or `COM3`)
- `type: string` — port type ("Usb", "Bluetooth", "Pci", "Unknown")
- `usbInfo?: { vid, pid, serial?, manufacturer?, product? }` — USB-specific fields when available
- `open(settings?: PortSettings): OpenPort` — open the port using optional settings

`OpenPort` (object returned from `open`) methods and settable properties:

- `write(data: Buffer | Uint8Array)` — enqueue bytes to be written to the port
- `onDataReceived = (err, data) => {}` — set a callback to receive incoming data (Buffer)
- `onWriteError = (err) => {}` — set a callback to receive write errors
- `close()` — close the port and stop the worker

`PortSettings` (optional) object fields:

- `baud_rate?: number` — default 115200
- `timeout_ms?: number` — read timeout in milliseconds (default 10)
- `data_bits?: 'Five'|'Six'|'Seven'|'Eight'`
- `parity?: 'None'|'Odd'|'Even'`
- `stop_bits?: 'One'|'Two'`
- `flow_control?: 'None'|'Software'|'Hardware'`

See `src/types.rs`, `src/ports.rs` and `src/open_port.rs` for the exact behavior and defaults.

## Example

```ts
import { list_ports } from 'native-serial';

const ports = list_ports();
if (!ports.length) {
  console.log('No serial ports found');
  return;
}

const port = ports[0];
console.log('Opening', port.path);

const open = port.open({ baud_rate: 115200 });

// onDataReceived receives (err, buffer)
open.onDataReceived((err, buf) => {
  if (err) {
    console.error('read error', err);
    return;
  }
  console.log('received', Buffer.from(buf).toString('hex'));
});

// optional: handle write errors
open.onWriteError(err => console.error('write error', err));

// write some bytes
open.write(Buffer.from([0x01, 0x02, 0x03]));

// ... later
open.close();
```

TypeScript users can use `index.d.ts` for typings that match the Rust-exported types.

## Build from source

Requirements:

- Rust toolchain (stable)
- Node.js (>= 12.22, see `package.json` engines)
- yarn or npm

Build the native addon locally:

```bash
yarn install
yarn build
# or with npm: npm run build
```

This runs the `napi` build tasks and produces the native binary in the project root
(for example `native-serial.linux-x64-gnu.node`).

## Testing and benchmarks

The repository contains a small test harness under `__test__/` and a `benchmark/` script.
Run tests with:

```bash
yarn test
```

Run the benchmark (requires the `tinybench` dev dependency):

```bash
yarn bench
```

## Contributing

Contributions are welcome. If you plan to change the public API, please open an issue or a PR
describing the change and its rationale.

When developing:

- Keep code formatted (Rust: `cargo fmt`, JS: `prettier`)
- Run tests before opening a PR

## License

This project is licensed under the MIT License — see the `LICENSE` file for details.
