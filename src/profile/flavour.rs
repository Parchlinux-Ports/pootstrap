use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlavourProfile {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub packages: Vec<String>,
    pub services: Vec<String>,
    pub environment_vars: Vec<(String, String)>,
}

impl FlavourProfile {
    pub fn builtin_flavours() -> Vec<Self> {
        vec![
            Self {
                name: "phosh".into(),
                display_name: "Phosh (GNOME Mobile)".into(),
                description: "Touch-friendly mobile shell built on GNOME, Phoc, and Wayland"
                    .into(),
                packages: vec![
                    "phosh".into(),
                    "phoc".into(),
                    "squeekboard".into(),
                    "feedbackd".into(),
                    "gnome-control-center".into(),
                    "epiphany".into(),
                    "calls".into(),
                    "chatty".into(),
                ],
                services: vec![
                    "phosh.service".into(),
                    "feedbackd.service".into(),
                    "NetworkManager.service".into(),
                    "bluetooth.service".into(),
                ],
                environment_vars: vec![("XDG_CURRENT_DESKTOP".into(), "Phosh:GNOME".into())],
            },
            Self {
                name: "plasma-mobile".into(),
                display_name: "Plasma Mobile (KDE)".into(),
                description: "Touch-optimized mobile user interface by KDE".into(),
                packages: vec![
                    "plasma-mobile".into(),
                    "plasma-nano".into(),
                    "maliit-keyboard".into(),
                    "angelfish".into(),
                    "spacebar".into(),
                    "kclock".into(),
                    "sddm".into(),
                ],
                services: vec![
                    "sddm.service".into(),
                    "NetworkManager.service".into(),
                    "bluetooth.service".into(),
                ],
                environment_vars: vec![("XDG_CURRENT_DESKTOP".into(), "KDE".into())],
            },
            Self {
                name: "sxmo".into(),
                display_name: "Sxmo (Simple Mobile)".into(),
                description:
                    "Lightweight mobile interface based on Sway, Wayland, and shell scripts".into(),
                packages: vec![
                    "sxmo-utils".into(),
                    "sway".into(),
                    "bemenu".into(),
                    "foot".into(),
                    "lisgd".into(),
                ],
                services: vec![
                    "sxmo.service".into(),
                    "NetworkManager.service".into(),
                ],
                environment_vars: vec![("XDG_CURRENT_DESKTOP".into(), "sway".into())],
            },
            Self {
                name: "minimal".into(),
                display_name: "Minimal (Console)".into(),
                description: "Console-only base system without graphical interface".into(),
                packages: vec![
                    "htop".into(),
                    "tmux".into(),
                    "openssh".into(),
                    "curl".into(),
                    "wget".into(),
                ],
                services: vec![
                    "sshd.service".into(),
                    "NetworkManager.service".into(),
                ],
                environment_vars: vec![],
            },
        ]
    }

    pub fn find(name: &str) -> Option<Self> {
        Self::builtin_flavours().into_iter().find(|f| f.name == name)
    }
}
