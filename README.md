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

Public exports (from the native addon):

- `listPorts(): Array<AvailablePort>` — synchronously list available serial ports

Types / classes:

- `AvailablePort`
  - `readonly path: string` — path to device (e.g. `/dev/ttyUSB0` or `COM3`)
  - `readonly type: string` — port type ("Usb", "Bluetooth", "Pci", "Unknown")
  - `readonly usb?: UsbInfo` — USB-specific fields when available
  - `open(onDataReceived: (data: Buffer) => void, onError: (err: Error | null) => void, settings?: PortSettings | null | undefined): OpenPort` — open the port and register callbacks

- `OpenPort` (returned by `AvailablePort.open`)
  - `write(data: Buffer): void` — enqueue bytes to be written to the port
  - `close(): void` — close the port and stop the worker

Enums (exported):

- `DataBits` — 'Five' | 'Six' | 'Seven' | 'Eight'
- `FlowControl` — 'None' | 'Software' | 'Hardware'
- `Parity` — 'None' | 'Odd' | 'Even'
- `StopBits` — 'One' | 'Two'

Settings and helper types:

- `PortSettings` (optional) object fields:
  - `baudRate?: number` — baud rate (defaults to 115200)
  - `timeoutMs?: number` — read timeout in milliseconds (default is 10ms)
  - `dataBits?: DataBits`
  - `parity?: Parity`
  - `stopBits?: StopBits`
  - `flowControl?: FlowControl`

- `UsbInfo`:
  - `readonly vid: number`
  - `readonly pid: number`
  - `readonly serial?: string`
  - `readonly manufacturer?: string`
  - `readonly product?: string`

## Example

```ts
import { listPorts } from 'native-serial';

const ports = listPorts();
if (!ports.length) {
  console.log('No serial ports found');
  process.exit(0);
}

const port = ports[0];
console.log('Opening', port.path);

// Open with callbacks and optional settings
const open = port.open(
  // onDataReceived: receives a Buffer
  buf => {
    console.log('received', buf.toString('hex'));
  },
  // onError: receives an Error or null
  err => {
    if (err) console.error('port error', err);
  },
  { baudRate: 115200 },
);

// write some bytes (Buffer)
open.write(Buffer.from([0x01, 0x02, 0x03]));

// ... later
open.close();
```

TypeScript users can rely on the shipped `index.d.ts` for types.

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

- Keep code formatted (`yarn format`)
- Run tests before opening a PR (`yarn test`)

## License

This project is licensed under the MIT License — see the `LICENSE` file for details.
