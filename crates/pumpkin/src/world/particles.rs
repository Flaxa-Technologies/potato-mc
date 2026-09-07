use pumpkin_data::particle::Particle;
use pumpkin_protocol::java::client::play::CParticle;
use pumpkin_util::math::vector3::Vector3;

use super::World;

impl World {
    /// Spawns a particle in the world for all currently connected players.
    pub fn spawn_particle(
        &self,
        position: Vector3<f64>,
        offset: Vector3<f32>,
        max_speed: f32,
        particle_count: i32,
        particle: Particle,
    ) {
        for player in self.players.load().iter() {
            player.spawn_particle(position, offset, max_speed, particle_count, particle);
        }
    }

    /// Spawns a cluster of particles in the world for all players in range.
    pub fn spawn_particles(
        &self,
        particle: pumpkin_data::particle::Particle,
        pos: Vector3<f64>,
        count: u32,
        offset: Vector3<f32>,
        max_speed: f32,
    ) {
        let packet = CParticle::new(
            false,
            false,
            pos,
            offset,
            max_speed,
            count as i32,
            (particle.to_id() as i32).into(),
            &[],
        );
        self.broadcast_packet_all(&packet);
    }
}
