import test from 'ava';
import serial from '../index.js';

test('module loads and exposes expected symbols', t => {
  t.truthy(serial, 'native binding should be defined');
  const expected = ['OpenPort', 'Port', 'DataBits', 'FlowControl', 'listPorts', 'Parity', 'StopBits'];
  for (const key of expected) {
    t.truthy(Object.prototype.hasOwnProperty.call(serial, key), `${key} should be exported`);
  }
});

test('listPorts returns an array', async t => {
  const ports = serial.listPorts();
  t.true(Array.isArray(ports), 'listPorts should return an array');
});

test('Port prototype exposes open method', t => {
  t.true(typeof serial.Port === 'function', 'Port should be a constructor');
  const hasOpen = typeof serial.Port?.prototype?.open === 'function';
  t.true(hasOpen, 'Port.prototype.open should be a function');
});
