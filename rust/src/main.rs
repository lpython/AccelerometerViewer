use bevy::{
    prelude::*,
    core::FixedTimestep,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    render::mesh::Mesh, transform
};
use bevy_serial::{SerialPlugin, SerialReadEvent, SerialWriteEvent};

// to write data to serial port periodically
// struct SerialWriteTimer(Timer);

const TIME_STEP: f32 = 1.0 / 30.0;

#[derive(Component)]
struct MyObject();

fn main() {
    App::new()
        //.add_plugins(MinimalPlugins)
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        // simply specify port name and baud rate for `SerialPlugin`
        .add_plugin(SerialPlugin::new("/dev/tty.usbserial-71D22653AC", 115200))
        .init_resource::<SerialStorage>()
        .add_system(read_serial)
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(TIME_STEP as f64))
                .with_system(box_movement_system),
        )
        .add_startup_system(setup)
        .run();
}


fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Mesh::from(shape::Box::new(50.0,10.0,50.0)));
    let material = materials.add(StandardMaterial {
        base_color: Color::PINK,
        ..Default::default()
    });
    
    commands.spawn_bundle(PbrBundle {
        mesh: mesh.clone(),
        material: material.clone(),
        transform: Transform::from_xyz((40.0f32) * 2.0, (20.0f32) * 2.0, 15.0),
        ..Default::default()
    }).insert(MyObject());

    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(80.0, 40.0, 120.0),
        ..Default::default()
    });
}

#[derive(Default)]
struct SerialStorage {
    buf: String,
    latest: Option<Vec3>
}

// reading event for serial port
fn read_serial(
    mut ev_serial: EventReader<SerialReadEvent>, 
    mut ss: ResMut<SerialStorage>
) {
    // you can get label of the port and received data buffer from `SerialReadEvent`
    for SerialReadEvent(label, buffer) in ev_serial.iter() {
        let s = match String::from_utf8(buffer.clone()) { 
            Ok(s) => s, 
            Err(_) => continue 
        };
        println!("received packet from {}: {}", label, s);
        ss.buf.push_str(&s);
    }

    let buf = ss.buf.clone();
    let mut iter = buf.rsplit('\n');
    if let Some(last) = iter.next() {
       if let Some(second) = iter.next() {
            ss.buf = last.to_string();

            let parts: Vec<&str> = second.split(' ').skip(3).collect();
            if parts.len() == 3 {
                let res = (
                    parts[0].parse(),
                    parts[1].parse(),
                    parts[2].parse(),
                );
                if let (Ok(x),Ok(y), Ok(z)) = res {
                    ss.latest = Some(Vec3::new(x,y,z));
                    dbg!(&ss.latest);
                }
            }
       } 
    }
}

fn box_movement_system(
    ss: Res<SerialStorage>,
    mut query: Query<(&MyObject, &mut Transform)>,
) {
    let v = match ss.latest {
        Some(v) => v,
        None => return
    };

    let (_, mut transform) = query.single_mut();

    let x = v.x;
    let mut z = v.y;

    //z += 0.5;

    // let mut zz = ((z + 1.0) * 1000.0) as i32;
    // zz = (zz + 500) % 1000;
    // z = zz as f32;
    // z = ((z / 1000.0) - 1.0);
    

    transform.rotation = Quat::from_rotation_x(x) * Quat::from_rotation_z(z);
    // dbg!(&transform.rotation);
}

// // writing event for serial port
// fn write_serial(
//     mut ev_serial: EventWriter<SerialWriteEvent>,
//     mut timer: ResMut<SerialWriteTimer>,
//     time: Res<Time>,
// ) {
//     if timer.0.tick(time.delta()).just_finished() {
//         // you can write to serial port via `SerialWriteEvent` with label and buffer to write
//         let buffer = b"Hello, bevy!";
//         ev_serial.send(SerialWriteEvent("COM5".to_string(), buffer.to_vec()));
//     }
// }