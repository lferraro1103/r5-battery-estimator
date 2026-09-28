const HID = require("node-hid");

const VID = 0x373e;
const PID = 0x0047;
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function parseBattery(report) {
  const bytes = Array.from(report);

  // Layout observed in ATTACK SHARK GAMING 1.0.2 getBatPer().
  if (bytes[1] === 0xa1 && bytes[4] === 0x02 && bytes[6] === 0x83) {
    return { charging: bytes[7] !== 0, percent: bytes[8], layout: "shifted" };
  }
  if (bytes[0] === 0xa1 && bytes[3] === 0x02 && bytes[5] === 0x83) {
    return { charging: bytes[6] !== 0, percent: bytes[7], layout: "normal" };
  }
  return null;
}

async function main() {
  const candidates = HID.devices(VID, PID).filter(
    (device) => device.usagePage >= 0xff00 || device.interface === 2,
  );

  if (candidates.length === 0) {
    throw new Error("R5 Ultra receiver found, but no vendor-defined HID interface is available");
  }

  const attempts = [];
  for (const candidate of candidates) {
    let device;
    try {
      device = new HID.HID(candidate.path);

      // node-hid includes report ID as byte 0. The following 64 bytes are the
      // same payload passed to WebHID sendFeatureReport(0, payload).
      const payload = new Array(65).fill(0);
      payload[3] = 0x02;
      payload[4] = 0x02;
      payload[6] = 0x83;

      const written = device.sendFeatureReport(payload);
      await delay(120);
      const report = device.getFeatureReport(0, 65);
      const parsed = parseBattery(report);
      attempts.push({
        interface: candidate.interface,
        usagePage: candidate.usagePage,
        usage: candidate.usage,
        written,
        response: Array.from(report.slice(0, 16)),
        parsed,
      });

      if (parsed && parsed.percent <= 100) {
        console.log(JSON.stringify({ device: candidate.product, ...parsed, attempts }, null, 2));
        return;
      }
    } catch (error) {
      attempts.push({
        interface: candidate.interface,
        usagePage: candidate.usagePage,
        usage: candidate.usage,
        error: error.message,
      });
    } finally {
      if (device) device.close();
    }
  }

  console.log(JSON.stringify({ parsed: null, attempts }, null, 2));
  process.exitCode = 2;
}

main().catch((error) => {
  console.error(error.stack || error.message);
  process.exitCode = 1;
});
