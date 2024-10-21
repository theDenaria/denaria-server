#![allow(dead_code)]
use std::{env, io, time::Duration};
mod constants;
mod ecs;
mod server;

use bevy_rapier3d::{
    plugin::{NoUserData, RapierPhysicsPlugin},
    render::RapierDebugRenderPlugin,
};
use ecs::systems::{
    debug::{
        look_debug_camera, move_debug_camera, set_debug_3d_render_camera, set_debug_metrics,
        set_debug_metrics_cam,
    },
    handle_events::{
        handle_character_movement, handle_disconnect_events, handle_look_events,
        handle_spawn_events,
    },
    handle_server::{handle_server_events, handle_server_messages, transport_send_packets},
    on_change::{on_spawn_change, on_transform_change},
    setup::{setup, setup_level},
};

use bevy::{
    app::ScheduleRunnerPlugin,
    diagnostic::{
        EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
        SystemInformationDiagnosticsPlugin,
    },
    prelude::*,
};

use iyes_perf_ui::prelude::*;

// #[tokio::main]
fn main() -> io::Result<()> {
    start_server();
    Ok(())
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum MySet {
    HandleServer,
    HandleGameEvents,
    Physics,
    HandleGameStateChanges,
}

fn start_server() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Optionally log an informational message
    info!("Starting server...");

    let enable_debug_metrics = env::var("DEBUG_METRICS").is_ok();
    let enable_debug_cam = env::var("DEBUG_CAM").is_ok();
    let mut app = App::new();

    if !enable_debug_metrics && !enable_debug_cam {
        app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f64(1.0 / 30.0),
        )));
    } else {
        app.add_plugins(DefaultPlugins)
            // .add_plugins(LogDiagnosticsPlugin::default())
            .add_plugins(FrameTimeDiagnosticsPlugin)
            .add_plugins(EntityCountDiagnosticsPlugin)
            .add_plugins(SystemInformationDiagnosticsPlugin)
            .add_plugins(PerfUiPlugin)
            .add_systems(PostStartup, set_debug_metrics);

        if enable_debug_cam {
            app.add_plugins(RapierDebugRenderPlugin::default())
                .add_systems(PostStartup, set_debug_3d_render_camera)
                .add_systems(Update, (move_debug_camera, look_debug_camera));
        } else {
            app.add_systems(PostStartup, set_debug_metrics_cam);
        }
    }

    app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_systems(Startup, (setup, setup_level).chain())
        .add_systems(
            PreUpdate,
            (handle_server_messages, handle_server_events).chain(),
        )
        .add_systems(
            Update,
            (
                (
                    handle_character_movement,
                    handle_look_events,
                    handle_spawn_events,
                    handle_disconnect_events,
                )
                    .in_set(MySet::HandleGameEvents)
                    .after(MySet::HandleServer),
                (on_transform_change, on_spawn_change)
                    .in_set(MySet::HandleGameStateChanges)
                    .after(MySet::HandleGameEvents),
                transport_send_packets.after(MySet::HandleGameStateChanges),
            ),
        );
    app.run();
}
