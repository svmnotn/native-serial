import { Bench } from 'tinybench';

import serial from '../index.js';

const b = new Bench();

b.add('list ports', () => {
  serial.listPorts();
});

await b.run();

console.table(b.table());
