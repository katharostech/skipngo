pub trait IntoBevy<T> {
    fn into_bevy(self) -> T;
}

pub trait IntoNav<T> {
    fn into_nav(self) -> T;
}

impl IntoBevy<bevy::math::Vec3> for navmesh::NavVec3 {
    fn into_bevy(self) -> bevy::math::Vec3 {
        bevy::math::Vec3::new(self.x, self.y, self.z)
    }
}
impl IntoNav<navmesh::NavVec3> for bevy::math::Vec3 {
    fn into_nav(self) -> navmesh::NavVec3 {
        navmesh::NavVec3 {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }
}
