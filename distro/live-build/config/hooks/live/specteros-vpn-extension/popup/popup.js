/**
 * SpecterOS VPN - Popup Script
 */

let vpnState = {};

// Initialize popup
async function init() {
  // Get current VPN state
  vpnState = await browser.runtime.sendMessage({ action: 'getState' });
  updateUI();
  
  // Listen for state changes
  browser.runtime.onMessage.addListener((message) => {
    if (message.action === 'stateChange') {
      vpnState = message.state;
      updateUI();
    }
  });
  
  // Event listeners
  document.getElementById('powerBtn').addEventListener('click', toggleVPN);
  document.getElementById('countrySelect').addEventListener('change', onCountryChange);
  document.getElementById('settingsLink').addEventListener('click', openSettings);
  
  // Update IP info if connected
  if (vpnState.connected) {
    updateIPInfo();
  }
}

// Update UI based on state
function updateUI() {
  const powerBtn = document.getElementById('powerBtn');
  const status = document.getElementById('status');
  const countrySelect = document.getElementById('countrySelect');
  const ipInfo = document.getElementById('ipInfo');
  
  // Power button
  if (vpnState.connected) {
    powerBtn.classList.add('connected');
    status.textContent = 'Connected via Tor';
    status.style.color = '#00c8aa';
    ipInfo.style.display = 'block';
  } else {
    powerBtn.classList.remove('connected');
    status.textContent = 'Disconnected';
    status.style.color = '#607080';
    ipInfo.style.display = 'none';
  }
  
  // Country selector
  countrySelect.value = vpnState.country;
  
  // Feature indicators
  document.getElementById('torFeature').classList.toggle('active', vpnState.torEnabled);
  document.getElementById('dnsFeature').classList.toggle('active', vpnState.connected);
  document.getElementById('killFeature').classList.toggle('active', vpnState.connected);
  document.getElementById('encryptFeature').classList.toggle('active', vpnState.connected);
}

// Toggle VPN
async function toggleVPN() {
  await browser.runtime.sendMessage({ action: 'toggle' });
}

// Country change
async function onCountryChange(e) {
  await browser.runtime.sendMessage({
    action: 'setCountry',
    country: e.target.value
  });
}

// Update IP information
async function updateIPInfo() {
  try {
    // Fetch IP from a privacy-respecting service
    const response = await fetch('https://api.ipify.org?format=json');
    const data = await response.json();
    
    document.getElementById('ipAddress').textContent = data.ip;
    document.getElementById('ipLocation').textContent = 'Tor Exit Node';
  } catch (e) {
    document.getElementById('ipAddress').textContent = 'Hidden';
    document.getElementById('ipLocation').textContent = 'Via Tor Network';
  }
}

// Open settings
function openSettings(e) {
  e.preventDefault();
  browser.runtime.openOptionsPage();
}

// Initialize on load
init();
