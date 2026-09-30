// Nushell's error types are large; this is dictated by the plugin API.
#![allow(clippy::result_large_err)]

mod command;
mod config;
mod converter;
#[cfg(feature = "probe")]
mod probe;
mod render;

use nu_plugin::{MsgPackSerializer, Plugin, PluginCommand, serve_plugin};

pub struct ChartonPlugin;

impl Plugin for ChartonPlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        #[allow(unused_mut)]
        let mut commands: Vec<Box<dyn PluginCommand<Plugin = Self>>> =
            vec![Box::new(command::Charton)];
        // `charton-probe` is a development diagnostic; enable it with
        // `--features probe` so it does not show up in users' `help commands`.
        #[cfg(feature = "probe")]
        commands.push(Box::new(probe::Probe));
        commands
    }
}

fn main() {
    serve_plugin(&ChartonPlugin, MsgPackSerializer);
}
