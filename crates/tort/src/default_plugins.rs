use tort_render::RenderPlugin;

use crate::{
    app::{PluginGroup, PluginGroupBuilder},
    asset::AssetPlugin,
    core::{FrameCountPlugin, TaskPoolPlugin, TypeRegistrationPlugin},
    diagnostic::DiagnosticsPlugin,
    input::InputPlugin,
    log::{Level, LogPlugin},
    time::TimePlugin,
    window::{MonitorSelection, Window, WindowPlugin, WindowPosition, WindowResolution},
    winit::WinitPlugin,
};

#[derive(Default)]
pub struct DefaultPlugins;

impl PluginGroup for DefaultPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(LogPlugin {
                level: Level::DEBUG,
                ..Default::default()
            })
            .add(TaskPoolPlugin::default())
            .add(TypeRegistrationPlugin::default())
            .add(FrameCountPlugin::default())
            .add(TimePlugin::default())
            .add(AssetPlugin {
                asset_folder: "../../assets".to_owned(),
                ..Default::default()
            })
            .add(DiagnosticsPlugin::default())
            .add(InputPlugin::default())
            .add(WindowPlugin {
                primary_window: Some(Window {
                    position: WindowPosition::Centered(MonitorSelection::Primary),
                    resolution: WindowResolution::new(1600.0, 900.0),
                    title: "tort".to_owned(),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .add(WinitPlugin::default())
            .add(RenderPlugin::default())
    }
}
