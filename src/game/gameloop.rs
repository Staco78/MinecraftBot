use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use parking_lot::RwLock;

use crate::{
    game::{Game, Vec3d},
    packets::{PlayerPosFlags, ReceiveError, SetPlayerPosition, send_packet_from_thread},
};

static SHOULD_RUN: AtomicBool = AtomicBool::new(true);

pub fn start_gameloop(game: Arc<RwLock<Game>>) -> JoinHandle<()> {
    thread::spawn(|| gameloop_(game))
}

#[allow(dead_code)]
pub fn stop_gameloop(handle: JoinHandle<()>) {
    SHOULD_RUN.store(false, Ordering::Relaxed);
    handle.join().unwrap();
}

fn gameloop_(game: Arc<RwLock<Game>>) {
    match gameloop(game) {
        Ok(_) => (),
        Err(e) => {
            panic!("{e}");
        }
    }
}

fn gameloop(game: Arc<RwLock<Game>>) -> Result<(), ReceiveError> {
    const TPS: u64 = 20;
    const TICK_TIME: Duration = Duration::from_nanos(1_000_000_000 / TPS);

    while SHOULD_RUN.load(Ordering::Relaxed) {
        let starting_time = Instant::now();

        game_logic(&game)?;

        let elapsed = starting_time.elapsed();
        if elapsed < TICK_TIME {
            thread::sleep(TICK_TIME - elapsed);
        }
    }

    Ok(())
}

fn game_logic(game: &RwLock<Game>) -> Result<(), ReceiveError> {
    let mut player_entity = game.read().player.entity.write_arc();

    let (sin, cos) = (player_entity.rotation.yaw as f64).to_radians().sin_cos();
    let look_at = Vec3d {
        x: -sin,
        y: 0.,
        z: cos,
    };

    let pos = player_entity.position;
    let new_pos = pos + look_at * 0.1;
    player_entity.position = new_pos;
    drop(player_entity);

    send_packet_from_thread(SetPlayerPosition {
        pos: new_pos,
        flags: PlayerPosFlags::empty(),
    })?;

    Ok(())
}
