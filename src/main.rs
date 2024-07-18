#![feature(trivial_bounds)]

use std::time::{Duration, Instant};

use bevy::{
    prelude::*,
    window::{Cursor, WindowLevel, WindowResolution},
};
use device_query::{DeviceQuery, DeviceState};
use rand::Rng;

#[derive(Component, Debug, Clone, Reflect)]
pub struct DQ {
    pub device_state: DeviceState,
    pub position: Vec2,
    pub t: f32,
    pub wander: bool,
    pub wander_pos: Vec2,
    pub last_clicked: Instant,
    pub window_size: Vec2,
    pub wandering_since: Instant,
}
unsafe impl Sync for DQ {}
unsafe impl Send for DQ {}

fn main() {
    let window = Window {
        // Enable transparent support for the window
        transparent: true,
        decorations: false,
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

#[derive(Component)]
struct Rat;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let texture = asset_server.load("rats/combined_rats.png");
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
        Rat,
    ));
    let device_state = DeviceState::new();

    commands.spawn(DQ {
        device_state,
        last_clicked: Instant::now(),
        wandering_since: Instant::now(),
        position: Vec2::ZERO,
        t: 0.0,
        wander: false,
        wander_pos: Vec2::ZERO,
        window_size: Vec2::ZERO,
    });
}

fn get_window(
    mut windows: Query<&mut Window>,
    mut dq: Query<&mut DQ>,
    mut sprite: Query<&mut Transform, With<Rat>>,
    buttons: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
) {
    let mut window = windows.get_single_mut().unwrap();

    let mut dq = dq.get_single_mut().unwrap();
    let m = dq.device_state.clone().get_mouse();

    // HEy
    let mouse = m.coords;

    let pos = &window.resolution;
    let target_x = mouse.0 as f32 - pos.width() / 2.;
    let target_y = mouse.1 as f32 - pos.height() / 2.;

    if target_x > dq.window_size.x {
        dq.window_size.x = target_x;
    }

    if target_y > dq.window_size.y {
        dq.window_size.y = target_y;
    }

    let mut target_pos = Vec2::new(target_x, target_y);

    if dq.wander {
        target_pos = dq.wander_pos;

        if dq.wandering_since.elapsed().as_millis() > 1000 * 12 {
            dq.wandering_since = Instant::now();
            let mut rng = rand::thread_rng();
            dq.wander_pos = Vec2::new(
                rng.gen_range(0..dq.window_size.x as i32) as f32,
                rng.gen_range(0..dq.window_size.y as i32) as f32,
            );
        }
    }

    let current_pos = match dq.position {
        pos if pos.x == 0.0 && pos.y == 0.0 => target_pos,
        pos => pos,
    };

    let difference = target_pos - current_pos;

    if difference.x.abs() > 1.2 {
        let mut sp = sprite.get_single_mut().unwrap();
        if difference.x > 0.0 {
            sp.scale.x = 6.0;
        } else {
            sp.scale.x = -6.0;
        }
    }

    if buttons.just_pressed(MouseButton::Left) {
        if Instant::now() - dq.last_clicked < Duration::from_millis(500) {
            let mut random = rand::thread_rng();
            dq.wander = true;
            dq.wandering_since = Instant::now();
            dq.wander_pos = Vec2::new(
                random.gen_range(0..dq.window_size.x as i32) as f32,
                random.gen_range(0..dq.window_size.y as i32) as f32,
            );
        } else {
            dq.wander = false;
        }
        dq.last_clicked = Instant::now();
    };

    if buttons.pressed(MouseButton::Left) {
        dq.t = (dq.t + time.delta_seconds() * 0.02).min(1.0);

        let new_pos = current_pos.lerp(target_pos, dq.t);

        window.position.set(new_pos.round().as_ivec2());
        dq.position = new_pos;
    } else {
        let direction = if difference.length() != 0.0 {
            difference.normalize()
        } else {
            Vec2::ZERO
        };

        let speed = match dq.wander {
            true => 120.0,
            false => 50.0,
        };

        let movement = direction * speed * time.delta_seconds();

        let new_pos = current_pos + movement;

        window.position.set(new_pos.round().as_ivec2());
        dq.position = new_pos;
        dq.t = 0.0;
    }
}
