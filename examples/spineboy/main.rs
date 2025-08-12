use bevy::prelude::*;

use bevy_spine::{SkeletonData, SpinePlugin};
use bullet::BulletPlugin;
use player::{PlayerPlugin, PlayerSpawnEvent};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            SpinePlugin::default(),
            PlayerPlugin,
            BulletPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut skeletons: ResMut<Assets<SkeletonData>>,
    mut player_spawn_events: EventWriter<PlayerSpawnEvent>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(Camera2d);

    let skeleton = SkeletonData::new_from_binary(
        asset_server.load("spineboy/export/spineboy-pro.skel"),
        asset_server.load("spineboy/export/spineboy.atlas"),
    );

    player_spawn_events.write(PlayerSpawnEvent {
        skeleton: skeletons.add(skeleton),
    });
}

mod bullet;
mod player;
