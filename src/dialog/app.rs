use anyhow::{Context, Result};
use crownui::{
    AppConfig, kit,
    prelude::{Anchor, KeyboardInteractivity, Layer, LayerOptions, TRANSPARENT},
};

use super::root::root;
use crate::request::DialogRequest;

const NAMESPACE: &str = "crownos-polkit-agent";

/// Shows `request` as a full-screen overlay that holds the keyboard until it exits.
pub fn run(request: DialogRequest) -> Result<()> {
    let layer = LayerOptions::new(NAMESPACE, Anchor::ALL)
        .layer(Layer::Overlay)
        .exclusive_zone(-1)
        .keyboard(KeyboardInteractivity::Exclusive);
    let config = AppConfig::new("Authentication Required")
        .app_id(NAMESPACE)
        .clear_color(TRANSPARENT)
        .font(kit::inter_font().to_vec(), Some(kit::INTER_FAMILY))
        .layer(layer);
    crownui::run(root(request), config).context("the authentication dialog stopped")
}
