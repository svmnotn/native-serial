import test from 'ava';
import serial from '../index';

test('module loads and exposes expected symbols', t => {
  t.truthy(serial, 'native binding should be defined');
  const expected = ['OpenPort', 'AvailablePort', 'DataBits', 'FlowControl', 'listPorts', 'Parity', 'StopBits'];
  for (const key of expected) {
    t.truthy(Object.prototype.hasOwnProperty.call(serial, key), `${key} should be exported`);
  }
});

test('listPorts returns an array', async t => {
  const ports = serial.listPorts();
  t.true(Array.isArray(ports), 'listPorts should return an array');
});

test('AvailablePort prototype exposes open method', t => {
  t.true(typeof serial.AvailablePort === 'function', 'AvailablePort should be a constructor');
  const hasOpen = typeof serial.AvailablePort?.prototype?.open === 'function';
  t.true(hasOpen, 'AvailablePort.prototype.open should be a function');
});
