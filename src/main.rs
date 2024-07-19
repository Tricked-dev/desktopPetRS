#![feature(trivial_bounds)]

use std::time::{Duration, Instant};

use bevy::{
    input::common_conditions::input_just_pressed,
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
    pub movement: Vec2,
    pub last_movement: Vec2,
    pub movement_timer: Timer,
}
unsafe impl Sync for DQ {}
unsafe impl Send for DQ {}

fn main() {
    let window = Window {
        // Enable transparent support for the window
        transparent: true,
        decorations: false,
        window_level: WindowLevel::AlwaysOnTop,
        resolution: WindowResolution::new(390.0, 243.0),
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
        .add_systems(Update, (execute_animations, get_window))
        .add_systems(
            Update,
            change_skin.run_if(input_just_pressed(MouseButton::Right)),
        )
        .run();
}

#[derive(Component, Debug)]
struct AnimationConfig {
    first_sprite_index: usize,
    last_sprite_index: usize,
    fps: u8,
    frame_timer: Timer,
    style: Style,
}

impl AnimationConfig {
    fn set_animation(&mut self, animation: Animations) {
        match animation {
            Animations::Walking => {
                self.first_sprite_index = 8;
                self.last_sprite_index = 15;
            }
            Animations::Flying => {
                self.first_sprite_index = 19;
                self.last_sprite_index = 20;
            }
            Animations::Idle => {
                self.first_sprite_index = 0;
                self.last_sprite_index = 6;
            }
        }
    }
}

enum Animations {
    Walking,
    Flying,
    Idle,
}

#[derive(Debug)]
enum Style {
    Crimson,
    House,
    Toxic,
}

impl Style {
    fn get_starting_point(&self) -> usize {
        match self {
            Style::Crimson => 0,
            Style::House => 81,
            Style::Toxic => 81 * 2,
        }
    }
}

impl AnimationConfig {
    fn new(first: usize, last: usize, fps: u8) -> Self {
        Self {
            first_sprite_index: first,
            last_sprite_index: last,
            fps,
            frame_timer: Self::timer_from_fps(fps),
            style: Style::Toxic,
        }
    }

    fn timer_from_fps(fps: u8) -> Timer {
        Timer::new(Duration::from_secs_f32(1.0 / (fps as f32)), TimerMode::Once)
    }
}

fn execute_animations(
    time: Res<Time>,
    mut query: Query<(&mut AnimationConfig, &mut TextureAtlas)>,
) {
    for (mut config, mut atlas) in &mut query {
        let additional = config.style.get_starting_point();
        config.frame_timer.tick(time.delta());
        if config.frame_timer.just_finished() {
            if atlas.index >= config.last_sprite_index + additional
                || atlas.index < (config.first_sprite_index + additional)
            {
                atlas.index = config.first_sprite_index + additional;
            } else {
                atlas.index += 1;
                config.frame_timer = AnimationConfig::timer_from_fps(config.fps);
            }
        }
    }
}

fn change_skin(mut query: Query<&mut AnimationConfig>) {
    for mut config in &mut query {
        config.style = match config.style {
            Style::Crimson => Style::House,
            Style::House => Style::Toxic,
            Style::Toxic => Style::Crimson,
        };
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

    let animation_config = AnimationConfig::new(0, 6, 10);
    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        SpriteBundle {
            transform: Transform::from_scale(Vec3::splat(6.0)),
            texture,
            ..default()
        },
        TextureAtlas {
            layout: texture_atlas_layout,
            index: animation_config.first_sprite_index,
        },
        Rat,
        animation_config,
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
        movement_timer: Timer::from_seconds(0.3, TimerMode::Repeating),
        movement: Vec2::ZERO,
        last_movement: Vec2::ZERO,
    });
}

fn get_window(
    mut windows: Query<&mut Window>,
    mut dq: Query<&mut DQ>,
    mut sprite: Query<&mut Transform, With<Rat>>,
    mut animation: Query<&mut AnimationConfig>,
    buttons: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
) {
    let mut window = windows.get_single_mut().unwrap();

    let mut dq = dq.get_single_mut().unwrap();
    let m = dq.device_state.clone().get_mouse();

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

    macro_rules! change_wander {
        () => {
            dq.wandering_since = Instant::now();
            let mut rng = rand::thread_rng();
            dq.wander_pos = Vec2::new(
                rng.gen_range(0..dq.window_size.x as i32) as f32,
                rng.gen_range(0..dq.window_size.y as i32) as f32,
            );
        };
    }

    if dq.wander {
        target_pos = dq.wander_pos;

        if dq.wandering_since.elapsed().as_millis() > 1000 * 12 {
            change_wander!();
        }
    }

    dq.movement_timer.tick(time.delta());
    if dq.movement_timer.just_finished() {
        dq.last_movement = dq.movement;
        dq.movement = Vec2::ZERO;
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

    let mut anim = animation.get_single_mut().unwrap();
    anim.frame_timer.set_mode(TimerMode::Repeating);
    if buttons.just_pressed(MouseButton::Left) {
        if Instant::now() - dq.last_clicked < Duration::from_millis(500) {
            dq.wander = true;
            change_wander!();
        } else {
            dq.wander = false;
        }
        anim.frame_timer.set_mode(TimerMode::Repeating);
        dq.last_clicked = Instant::now();
    };

    if buttons.pressed(MouseButton::Left) {
        dq.t = (dq.t + time.delta_seconds() * 0.02).min(1.0);

        let new_pos = current_pos.lerp(target_pos, dq.t);

        if dq.last_movement.x < 0.12 {
            anim.set_animation(Animations::Idle);
        } else {
            anim.set_animation(Animations::Flying);
        }

        dq.movement += (new_pos - current_pos).abs();

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

        dq.movement += difference.abs();

        if dq.last_movement.x < 20.0 || dq.last_movement.y < 20.0 {
            anim.set_animation(Animations::Idle);
        } else {
            anim.set_animation(Animations::Walking);
        }

        window.position.set(new_pos.round().as_ivec2());
        dq.position = new_pos;
        dq.t = 0.0;
    }
}
