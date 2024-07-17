#![feature(trivial_bounds)]

use std::sync::{Arc, Mutex};

use bevy::{
    prelude::*,
    window::{Cursor, WindowLevel, WindowResolution},
};
use device_query::{DeviceQuery, DeviceState};

#[derive(Component, Default, Debug, Clone, Reflect)]
pub struct DQ {
    pub device_state: DeviceState,
    pub position: IVec2,
}
unsafe impl Sync for DQ {}
unsafe impl Send for DQ {}

fn main() {
    let window = Window {
        // Enable transparent support for the window
        transparent: true,
        decorations: true,
        window_level: WindowLevel::AlwaysOnTop,
        resolution: WindowResolution::new(262.0, 243.0),
        cursor: Cursor {
            // Allow inputs to pass through to apps behind this app.
            ..default()
        },
        ..default()
    };

    App::new()
        .insert_resource(ClearColor(Color::NONE))
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(window),
                    ..default()
                })
                .set(AssetPlugin {
                    mode: AssetMode::Processed,
                    ..default()
                }),
        )
        .add_systems(Startup, setup)
        .add_systems(Update, (animate_sprite, get_window))
        .run();
}

#[derive(Component)]
struct AnimationIndices {
    first: usize,
    last: usize,
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut TextureAtlas)>,
) {
    for (indices, mut timer, mut atlas) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished() {
            atlas.index = if atlas.index == indices.last {
                indices.first
            } else {
                atlas.index + 1
            };
        }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let texture = asset_server.load("rats/combined_rats.png");
    dbg!(&texture);
    let layout = TextureAtlasLayout::from_grid(UVec2::new(62, 44), 9, 27, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);
    // Use only the subset of sprites in the sheet that make up the run animation
    let animation_indices = AnimationIndices { first: 2, last: 6 };
    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        SpriteBundle {
            transform: Transform::from_scale(Vec3::splat(6.0)),
            texture,
            ..default()
        },
        TextureAtlas {
            layout: texture_atlas_layout,
            index: animation_indices.first,
        },
        animation_indices,
        AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
    ));
    let device_state = DeviceState::new();

    commands.spawn(DQ {
        device_state: device_state,
        position: IVec2::new(0, 0),
    });
}

fn get_window(
    mut windows: Query<&mut Window>,
    mut dq: Query<&mut DQ>,
    buttons: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
) {
    let mut window = windows.get_single_mut().unwrap();

    println!("Window size was: {},{}", window.width(), window.height());
    let mut dq = dq.get_single_mut().unwrap();
    let m = dq.device_state.clone().get_mouse();

    if buttons.pressed(MouseButton::Left) {
        let mouse = m.coords;

        let pos = &window.resolution;
        let target_x = mouse.0 - pos.width() as i32 / 2;
        let target_y = mouse.1 - pos.height() as i32 / 2;

        // Get current position
        let current_pos = dq.position.as_vec2();

        // Calculate the direction to move
        let direction = Vec2::new(target_x as f32, target_y as f32) - current_pos;

        // Set a speed for the movement (adjust as needed)
        let speed = 500.0; // pixels per second

        // Calculate the movement based on delta time
        let movement = direction.normalize() * speed * time.delta_seconds();

        // Update the position
        let new_pos = current_pos + movement;
        let pos = IVec2::new(new_pos.x as i32, new_pos.y as i32);
        // Set the new position
        window.position.set(pos);
        dq.position = pos;
    }
}

#[derive(Component)]
enum Direction {
    Up,
    Down,
}

/// The sprite is animated by changing its translation depending on the time that has passed since
/// the last frame.
fn sprite_movement(time: Res<Time>, mut sprite_position: Query<(&mut Direction, &mut Transform)>) {
    for (mut logo, mut transform) in &mut sprite_position {
        match *logo {
            Direction::Up => transform.translation.y += 150. * time.delta_seconds(),
            Direction::Down => transform.translation.y -= 150. * time.delta_seconds(),
        }

        if transform.translation.y > 200. {
            *logo = Direction::Down;
        } else if transform.translation.y < -200. {
            *logo = Direction::Up;
        }
    }
}
