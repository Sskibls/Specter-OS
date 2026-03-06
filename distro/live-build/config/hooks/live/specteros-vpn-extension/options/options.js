/**
 * SpecterOS VPN - Options Script
 */

// Load saved settings
async function loadSettings() {
  const settings = await browser.storage.local.get('vpnSettings');
  if (settings.vpnSettings) {
    document.getElementById('autoConnect').checked = settings.vpnSettings.autoConnect || false;
    document.getElementById('killSwitch').checked = settings.vpnSettings.killSwitch || true;
    document.getElementById('dnsLeak').checked = settings.vpnSettings.dnsLeak || true;
    document.getElementById('torEnabled').checked = settings.vpnSettings.torEnabled || true;
    document.getElementById('strictNodes').checked = settings.vpnSettings.strictNodes || false;
    document.getElementById('showSpeed').checked = settings.vpnSettings.showSpeed || false;
    document.getElementById('notifications').checked = settings.vpnSettings.notifications || true;
  }
}

// Save settings
async function saveSettings() {
  const settings = {
    autoConnect: document.getElementById('autoConnect').checked,
    killSwitch: document.getElementById('killSwitch').checked,
    dnsLeak: document.getElementById('dnsLeak').checked,
    torEnabled: document.getElementById('torEnabled').checked,
    strictNodes: document.getElementById('strictNodes').checked,
    showSpeed: document.getElementById('showSpeed').checked,
    notifications: document.getElementById('notifications').checked
  };
  
  await browser.storage.local.set({ vpnSettings: settings });
  
  // Show save notification
  alert('Settings saved!');
}

document.getElementById('saveBtn').addEventListener('click', saveSettings);

// Load on init
loadSettings();
