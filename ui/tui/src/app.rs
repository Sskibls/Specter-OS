// PhantomKernel TUI - Application State

use anyhow::Result;

pub type AppResult<T> = Result<T>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Dashboard,
    Shards,
    Network,
    Settings,
}

impl Tab {
    pub fn titles() -> Vec<&'static str> {
        vec!["Dashboard", "Shards", "Network", "Settings"]
    }
}

#[derive(Debug, Clone)]
pub struct Shard {
    pub name: String,
    pub status: ShardStatus,
    pub network: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShardStatus {
    Running,
    Stopped,
    Locked,
}

pub struct App {
    pub running: bool,
    pub current_tab: Tab,
    pub shards: Vec<Shard>,
    pub network_status: NetworkStatus,
    pub panic_mode: bool,
    pub mask_mode: bool,
    pub travel_mode: bool,
    pub kill_switch: bool,
    pub audit_events: Vec<String>,
}

pub struct NetworkStatus {
    pub status: String,
    pub upload: f64,
    pub download: f64,
    pub route: String,
    pub leak_detected: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            running: true,
            current_tab: Tab::Dashboard,
            shards: vec![
                Shard { name: "work".to_string(), status: ShardStatus::Stopped, network: "Direct".to_string() },
                Shard { name: "anon".to_string(), status: ShardStatus::Stopped, network: "Tor".to_string() },
                Shard { name: "burner".to_string(), status: ShardStatus::Stopped, network: "Tor".to_string() },
                Shard { name: "lab".to_string(), status: ShardStatus::Stopped, network: "Isolated".to_string() },
            ],
            network_status: NetworkStatus {
                status: "Secure".to_string(),
                upload: 0.0,
                download: 0.0,
                route: "eth0".to_string(),
                leak_detected: false,
            },
            panic_mode: false,
            mask_mode: false,
            travel_mode: false,
            kill_switch: false,
            audit_events: vec!["System initialized".to_string()],
        }
    }

    pub fn tick(&mut self) {
        // Simulate traffic updates
        if !self.kill_switch {
            self.network_status.upload = (self.network_status.upload * 0.9) + 5.0;
            self.network_status.download = (self.network_status.download * 0.9) + 25.0;
        } else {
            self.network_status.upload = 0.0;
            self.network_status.download = 0.0;
        }
    }

    pub fn next_tab(&mut self) {
        let tabs = Tab::titles();
        let current = tabs.iter().position(|&t| t == self.tab_name()).unwrap_or(0);
        let next = (current + 1) % tabs.len();
        self.set_tab(next);
    }

    pub fn previous_tab(&mut self) {
        let tabs = Tab::titles();
        let current = tabs.iter().position(|&t| t == self.tab_name()).unwrap_or(0);
        let prev = if current == 0 { tabs.len() - 1 } else { current - 1 };
        self.set_tab(prev);
    }

    pub fn set_tab(&mut self, index: usize) {
        self.current_tab = match index {
            0 => Tab::Dashboard,
            1 => Tab::Shards,
            2 => Tab::Network,
            _ => Tab::Settings,
        };
    }

    pub fn tab_name(&self) -> &'static str {
        match self.current_tab {
            Tab::Dashboard => "Dashboard",
            Tab::Shards => "Shards",
            Tab::Network => "Network",
            Tab::Settings => "Settings",
        }
    }

    pub fn activate_panic(&mut self) {
        self.panic_mode = true;
        self.kill_switch = true;
        for shard in &mut self.shards {
            shard.status = ShardStatus::Locked;
        }
        self.audit_events.insert(0, "⚠️ PANIC MODE ACTIVATED".to_string());
    }

    pub fn toggle_mask_mode(&mut self) {
        self.mask_mode = !self.mask_mode;
        let msg = if self.mask_mode { "🎭 Mask mode enabled" } else { "Mask mode disabled" };
        self.audit_events.insert(0, msg.to_string());
    }

    pub fn toggle_travel_mode(&mut self) {
        self.travel_mode = !self.travel_mode;
        let msg = if self.travel_mode { "✈️ Travel mode enabled" } else { "Travel mode disabled" };
        self.audit_events.insert(0, msg.to_string());
    }

    pub fn toggle_kill_switch(&mut self) {
        self.kill_switch = !self.kill_switch;
        self.network_status.status = if self.kill_switch { "KILL SWITCH" } else { "Secure" }.to_string();
        let msg = if self.kill_switch { "🚫 Kill switch ON" } else { "Kill switch OFF" };
        self.audit_events.insert(0, msg.to_string());
    }

    pub fn refresh(&mut self) {
        self.audit_events.insert(0, "🔄 Refreshed".to_string());
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
