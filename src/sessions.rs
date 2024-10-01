use std::time::Duration;

use bevy::{
    app::ScheduleRunnerPlugin,
    diagnostic::{
        EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
        SystemInformationDiagnosticsPlugin,
    },
    prelude::*,
};
use bevy_rapier3d::{
    plugin::{NoUserData, RapierPhysicsPlugin},
    render::RapierDebugRenderPlugin,
};
use crossbeam::channel::{Receiver, Sender};
use iyes_perf_ui::PerfUiPlugin;

use crate::{
    ecs::systems::{
        debug::{
            look_debug_camera, move_debug_camera, set_debug_3d_render_camera, set_debug_metrics,
            set_debug_metrics_cam,
        },
        handle_events::{
            handle_character_movement, handle_connect_events, handle_disconnect_events,
            handle_look_events,
        },
        handle_server::{handle_outgoing_messages, handle_server_events, handle_server_messages},
        on_change::on_transform_change,
        setup::{setup, setup_level},
    },
    server::{
        connection::ConnectionConfig,
        server::DenariaServer,
        transport::transport::{FromDenariaServerMessage, ToDenariaServerMessage},
    },
};

pub fn new_session(
    to_transport_server_tx: Sender<FromDenariaServerMessage>,
    from_transport_server_rx: Receiver<ToDenariaServerMessage>,
) {
    tracing::info!("Creating new session");

    let server = DenariaServer::new(
        ConnectionConfig::default(),
        from_transport_server_rx,
        to_transport_server_tx,
    );

    let mut app = App::new();

    app.insert_resource(server);

    let enable_debug_metrics =
        std::env::var("ENABLE_DEBUG_METRICS").is_ok_and(|v| v.to_lowercase() == "true");
    let enable_debug_cam =
        std::env::var("ENABLE_DEBUG_CAM").is_ok_and(|v| v.to_lowercase() == "true");

    if !enable_debug_metrics && !enable_debug_cam {
        app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f64(1.0 / 30.0),
        )));
    } else {
        app.add_plugins(DefaultPlugins)
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
            (handle_server_events, handle_server_messages).chain(),
        )
        .add_systems(PostUpdate, handle_outgoing_messages)
        .add_systems(
            Update,
            (
                (
                    handle_character_movement,
                    handle_look_events,
                    handle_connect_events,
                    handle_disconnect_events,
                )
                    .in_set(MySet::HandleGameEvents),
                (on_transform_change).after(MySet::HandleGameEvents),
            ),
        );

    app.run();
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, SystemSet)]
pub enum MySet {
    HandleGameEvents,
}
