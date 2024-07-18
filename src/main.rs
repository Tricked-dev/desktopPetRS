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
    pub t: f32,
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
        t: 0.0,
    });
}

fn get_window(
    mut windows: Query<&mut Window>,
    mut dq: Query<&mut DQ>,
    buttons: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
) {
    let mut window = windows.get_single_mut().unwrap();

    // println!("Window size was: {},{}", window.width(), window.height());
    let mut dq = dq.get_single_mut().unwrap();
    let m = dq.device_state.clone().get_mouse();

    if buttons.pressed(MouseButton::Left) {
        dq.t = (dq.t + time.delta_seconds() * 0.02).min(1.0);
        let mouse = m.coords;

        let pos = &window.resolution;
        let target_x = mouse.0 - pos.width() as i32 / 2;
        let target_y = mouse.1 - pos.height() as i32 / 2;
        let target_pos = Vec2::new(target_x as f32, target_y as f32);

        let current_pos = match dq.position.as_vec2() {
            pos if pos.x == 0.0 && pos.y == 0.0 => Vec2::new(target_x as f32, target_y as f32),
            pos => pos,
        };

        let new_pos = current_pos.lerp(target_pos, dq.t).as_ivec2();

        window.position.set(new_pos);
        dq.position = new_pos;
    } else {
        let mouse = m.coords;

        // Get the window resolution
        let pos = &window.resolution;
        let target_x = mouse.0 - pos.width() as i32 / 2;
        let target_y = mouse.1 - pos.height() as i32 / 2;

        // Create a target position vector
        let target_pos = Vec2::new(target_x as f32, target_y as f32);

        // Get the current position or initialize it to the target position if it's at the origin
        let current_pos = match dq.position.as_vec2() {
            pos if pos.x == 0.0 && pos.y == 0.0 => target_pos,
            pos => pos,
        };

        // Calculate the direction vector from the current position to the target position
        let direction = target_pos - current_pos;
        let direction = if direction.length() != 0.0 {
            direction.normalize()
        } else {
            Vec2::ZERO
        };

        // Calculate the movement vector based on direction, speed (0.5), and delta time
        let movement = direction * 0.5 * time.delta_seconds();

        // Update the new position by adding the movement vector to the current position
        let new_pos = current_pos + movement;

        // Convert the new position to integer coordinates
        let new_pos_ivec = new_pos.as_ivec2();

        // Set the window position to the new position
        window.position.set(new_pos_ivec);

        // Update the entity's position
        dq.position = new_pos_ivec;
        dq.t = 0.0;
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
