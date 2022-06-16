use bevy::prelude::*;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(character_movement)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn a flat and ugly terrain
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 10.0 })),
        material: materials.add(Color::rgb(0.6, 8.0, 0.6).into()),
        ..default()
    });

    // Spawn a fat and ugly character
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Capsule {
                depth: 0.5,
                radius: 0.125,
                ..default()
            })),
            material: materials.add(Color::rgb(0.6, 0.6, 0.6).into()),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        })
        .insert(Character);

    // Spawn a camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.0, 2.0, -5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

#[derive(Component)]
struct Character;

fn character_movement(
    input: Res<Input<KeyCode>>,
    time: Res<Time>,
    mut char_query: Query<&mut Transform, With<Character>>,
) {
    let mut move_dir = Vec2::default();

    if input.pressed(KeyCode::A) {
        move_dir.x += 1.0;
    } else if input.pressed(KeyCode::D) {
        move_dir.x -= 1.0;
    }

    if input.pressed(KeyCode::W) {
        move_dir.y += 1.0;
    } else if input.pressed(KeyCode::S) {
        move_dir.y -= 1.0;
    }

    let length = move_dir.length();

    if length > f32::EPSILON {
        //Prevent from moving faster on diagnals
        move_dir.x /= length;
        move_dir.y /= length;

        let mut transform = char_query.single_mut();
        transform.translation.x += move_dir.x * time.delta_seconds();
        transform.translation.z += move_dir.y * time.delta_seconds();
    }
}
