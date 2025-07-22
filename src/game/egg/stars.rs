use std::f32::consts::*;

use bevy::prelude::*;
use bevy_rand::prelude::*;
use rand_core::RngCore;

use super::*;

pub fn egg_stars_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Egg), setup_stars)
        .add_systems(
            FixedUpdate,
            (update_stars_position, despawn_respawn_stars).run_if(in_state(EggState::Ready)),
        );
}

const STARBOX_RADIUS: f32 = 32.0;

const MAX_STAR_AMOUNT: usize = 1024;
const BACK_STAR_AMOUNT: usize = 768;

const MIN_STAR_SPEED: f32 = 0.01;
const MAX_STAR_SPEED: f32 = 0.4;

const MIN_STAR_HEIGHT: f32 = -32.0;
const MAX_STAR_HEIGHT: f32 = 128.0;

const LUMINANCE_LEVELS: usize = 4;
const MIN_STAR_LUMINANCE: f32 = 4.0;
const MAX_STAR_LUMINANCE: f32 = 512.0;

#[derive(Debug, Component)]
struct StarRoot;

#[derive(Debug, Component)]
struct StarStatic;

#[derive(Debug, Component)]
// Star with parallax speed
struct Star {
    speed: f32,
}

#[derive(Debug, Component)]
// Star mesh and materials for each levels of luminance
struct StarResources {
    mesh:      Handle<Mesh>,
    materials: [Handle<StandardMaterial>; LUMINANCE_LEVELS],
}

struct StarInfo {
    speed:     f32,
    lum_level: usize,
    transform: Transform,
}

fn setup_stars(
    mut commands: Commands,
    mut rng: GlobalEntropy<WyRand>,
    asset_server: Res<AssetServer>,
) {
    info!("Spawning stars");

    let lum_increment = (MAX_STAR_LUMINANCE - MIN_STAR_LUMINANCE) / LUMINANCE_LEVELS as f32;
    let resources = StarResources {
        mesh:      asset_server.add(Circle::new(0.02).into()),
        materials: (0..LUMINANCE_LEVELS)
            .map(|level| {
                let lum = lum_increment * level as f32;
                asset_server.add(StandardMaterial {
                    emissive: LinearRgba::rgb(lum, lum, lum),
                    ..default()
                })
            })
            .collect::<Vec<Handle<StandardMaterial>>>()
            .try_into()
            .unwrap(),
    };

    commands
        .spawn((
            OnEggScene,
            StarRoot,
            Name::new("Stars"),
            Transform::default(),
            Visibility::default(),
        ))
        .with_children(|parent| {
            (0..MAX_STAR_AMOUNT)
                .map(|i| generate_star(&mut rng, i))
                .for_each(|info| {
                    let StarInfo {
                        speed,
                        lum_level,
                        transform,
                    } = info;

                    let new_star = parent
                        .spawn((
                            Mesh3d(resources.mesh.clone_weak()),
                            MeshMaterial3d(resources.materials[lum_level].clone_weak()),
                            transform,
                        ))
                        .id();

                    let commands = parent.commands_mut();
                    match info.speed == 0.0 {
                        true => commands.entity(new_star).insert(StarStatic),
                        false => commands.entity(new_star).insert(Star { speed }),
                    };
                })
        })
        .insert(resources);
}

fn update_stars_position(mut stars: Query<(&Star, &mut Transform)>) {
    stars
        .iter_mut()
        .for_each(|(Star { speed }, mut transform)| {
            transform.translation.y -= speed;
        });
}

fn despawn_respawn_stars(
    stars: Query<(Entity, &Transform), With<Star>>,
    star_root: Single<(Entity, &StarResources), With<StarRoot>>,
    mut rng: GlobalEntropy<WyRand>,
    mut commands: Commands,
) {
    let (star_root, resources) = star_root.into_inner();
    stars
        .iter()
        .filter_map(|(entity, transform)| {
            (transform.translation.y <= MIN_STAR_HEIGHT).then_some(entity)
        })
        .for_each(|entity| {
            commands.entity(entity).despawn();

            let StarInfo {
                speed,
                lum_level,
                transform,
            } = generate_star(&mut rng, MAX_STAR_AMOUNT);

            commands.entity(star_root).with_child((
                Star { speed },
                Mesh3d(resources.mesh.clone_weak()),
                MeshMaterial3d(resources.materials[lum_level].clone_weak()),
                transform,
            ));
        });
}

fn generate_star(rng: &mut Entropy<WyRand>, count: usize) -> StarInfo {
    // Star's angle on the half-cylinder skybox
    let angle = random_range(rng, 0.0, PI);

    // Static stars
    let speed = match count < BACK_STAR_AMOUNT {
        true => 0.0,
        false => random_range(rng, MIN_STAR_SPEED.sqrt(), MAX_STAR_SPEED.sqrt()).powi(2),
    };

    // Initial stars
    let height = match count < MAX_STAR_AMOUNT {
        true => random_range(rng, MIN_STAR_HEIGHT, MAX_STAR_HEIGHT),
        false => MAX_STAR_HEIGHT,
    };

    let lum_level = rng.next_u32() as usize % LUMINANCE_LEVELS;

    let transform = Transform::from_xyz(
        STARBOX_RADIUS * angle.cos(),
        height,
        STARBOX_RADIUS * angle.sin(),
    )
    .with_rotation(Quat::from_rotation_y(0.75 * TAU - angle));

    StarInfo {
        speed,
        lum_level,
        transform,
    }
}
