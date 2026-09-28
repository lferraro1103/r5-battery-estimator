const HID = require("node-hid");

const candidates = HID.devices().filter((device) =>
  [0x373e, 0x3151, 0x12cf].includes(device.vendorId),
);

console.log(JSON.stringify(candidates, null, 2));
