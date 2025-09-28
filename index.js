// @ts-nocheck

const { DataBits, FlowControl, OpenPort, Parity, StopBits, listPorts: lp, AvailablePort } = require('./build.js');

function listPorts() {
  return lp().map(p => ({
    open: (onDataReceived, onError, settings) => p.open(onDataReceived, onError, settings),
    path: p.path,
    type: p.type, 
    usb: p.usb
  }));
}

module.exports = {
  DataBits,
  FlowControl,
  OpenPort,
  Parity,
  StopBits,
  listPorts,
  AvailablePort,
};
