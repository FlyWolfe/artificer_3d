use avian3d::prelude::PhysicsLayer;

#[derive(PhysicsLayer)]
pub enum GameLayer {
    Default,
    Player,
    Enemy,
    Ground,
    Projectile,
}
