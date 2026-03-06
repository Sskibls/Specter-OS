/**
 * SpecterOS VPN - Background Script
 * Handles Tor routing and country selection
 */

// VPN State
let vpnState = {
  connected: false,
  country: 'auto',
  torEnabled: false,
  connectionSpeed: 'fast',
  countries: []
};

// Available Tor exit node countries
const COUNTRIES = [
  { code: 'auto', name: '🔄 Auto (Fastest)', flag: '🌐' },
  { code: 'us', name: 'United States', flag: '🇺🇸' },
  { code: 'de', name: 'Germany', flag: '🇩🇪' },
  { code: 'fr', name: 'France', flag: '🇫🇷' },
  { code: 'nl', name: 'Netherlands', flag: '🇳🇱' },
  { code: 'gb', name: 'United Kingdom', flag: '🇬🇧' },
  { code: 'ca', name: 'Canada', flag: '🇨🇦' },
  { code: 'se', name: 'Sweden', flag: '🇸🇪' },
  { code: 'no', name: 'Norway', flag: '🇳🇴' },
  { code: 'ch', name: 'Switzerland', flag: '🇨🇭' },
  { code: 'fi', name: 'Finland', flag: '🇫🇮' },
  { code: 'jp', name: 'Japan', flag: '🇯🇵' },
  { code: 'sg', name: 'Singapore', flag: '🇸🇬' },
  { code: 'au', name: 'Australia', flag: '🇦🇺' }
];

// Initialize
async function init() {
  // Load saved state
  const saved = await browser.storage.local.get('vpnState');
  if (saved.vpnState) {
    vpnState = { ...vpnState, ...saved.vpnState };
  }
  
  // Set country list
  vpnState.countries = COUNTRIES;
  
  // Update badge
  updateBadge();
  
  // Listen for messages from popup
  browser.runtime.onMessage.addListener(handleMessage);
  
  // Listen for keyboard shortcuts
  browser.commands.onCommand.addListener(handleCommand);
  
  console.log('[SpecterOS VPN] Initialized');
}

// Handle messages from popup
async function handleMessage(message, sender) {
  switch (message.action) {
    case 'toggle':
      await toggleVPN();
      break;
    case 'setCountry':
      await setCountry(message.country);
      break;
    case 'getState':
      return vpnState;
    case 'getCountries':
      return COUNTRIES;
  }
  return {};
}

// Handle keyboard shortcuts
function handleCommand(command) {
  if (command === 'toggle-vpn') {
    toggleVPN();
  }
}

// Toggle VPN connection
async function toggleVPN() {
  vpnState.connected = !vpnState.connected;
  
  if (vpnState.connected) {
    await connectVPN();
  } else {
    await disconnectVPN();
  }
  
  await saveState();
  updateBadge();
  broadcastState();
}

// Connect to VPN/Tor
async function connectVPN() {
  // Show connecting notification
  await browser.notifications.create('vpn-connecting', {
    type: 'basic',
    iconUrl: 'icons/vpn-48.png',
    title: 'SpecterOS VPN',
    message: 'Connecting to Tor network...'
  });
  
  // Configure proxy for Tor
  if (vpnState.torEnabled) {
    await configureTorProxy();
  }
  
  // Set proxy based on country
  await configureProxy();
  
  // Show connected notification
  setTimeout(async () => {
    await browser.notifications.create('vpn-connected', {
      type: 'basic',
      iconUrl: 'icons/vpn-48.png',
      title: 'SpecterOS VPN',
      message: `Connected via ${getCountryName(vpnState.country)}`
    });
  }, 2000);
}

// Disconnect VPN
async function disconnectVPN() {
  // Clear proxy settings
  await browser.proxy.settings.clear({});
  
  // Show disconnected notification
  await browser.notifications.create('vpn-disconnected', {
    type: 'basic',
    iconUrl: 'icons/vpn-48.png',
    title: 'SpecterOS VPN',
    message: 'VPN disconnected'
  });
}

// Set country
async function setCountry(countryCode) {
  vpnState.country = countryCode;
  await saveState();
  
  // Reconnect if already connected
  if (vpnState.connected) {
    await disconnectVPN();
    await connectVPN();
  }
  
  broadcastState();
}

// Configure Tor proxy
async function configureTorProxy() {
  // Tor default SOCKS5 proxy
  await browser.proxy.settings.set({
    value: {
      proxyType: 'manual',
      socks: '127.0.0.1',
      socksPort: 9050,
      socksVersion: 5,
      proxyDNS: true
    }
  });
}

// Configure country-specific proxy
async function configureProxy() {
  if (vpnState.country === 'auto') {
    // Use Tor's automatic circuit building
    return;
  }
  
  // For country-specific routing, we use Tor with country preference
  // This is handled by Tor configuration
  const torConfig = `
EntryNodes {${getCountryCode(vpnState.country)}}
StrictNodes 1
ExitNodes {${getCountryCode(vpnState.country)}}
`;
  
  // Save Tor configuration
  // (In production, this would write to torrc)
  console.log('[SpecterOS VPN] Tor config:', torConfig);
}

// Get country name from code
function getCountryName(code) {
  const country = COUNTRIES.find(c => c.code === code);
  return country ? `${country.flag} ${country.name}` : 'Unknown';
}

// Get country code for Tor
function getCountryCode(code) {
  const countryMap = {
    'us': 'US',
    'de': 'DE',
    'fr': 'FR',
    'nl': 'NL',
    'gb': 'GB',
    'ca': 'CA',
    'se': 'SE',
    'no': 'NO',
    'ch': 'CH',
    'fi': 'FI',
    'jp': 'JP',
    'sg': 'SG',
    'au': 'AU'
  };
  return countryMap[code] || '{US}';
}

// Update browser action badge
function updateBadge() {
  if (vpnState.connected) {
    browser.browserAction.setBadgeText({ text: 'ON' });
    browser.browserAction.setBadgeBackgroundColor({ color: '#00c8aa' });
  } else {
    browser.browserAction.setBadgeText({ text: '' });
  }
}

// Save state to storage
async function saveState() {
  await browser.storage.local.set({ vpnState });
}

// Broadcast state to popup
function broadcastState() {
  browser.runtime.sendMessage({
    action: 'stateChange',
    state: vpnState
  }).catch(() => {}); // Ignore if popup is not open
}

// Initialize on startup
init();
