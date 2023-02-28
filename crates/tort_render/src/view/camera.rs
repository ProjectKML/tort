use dolly::prelude::{CameraRig, LeftHanded, Position, Smooth, YawPitch};
use tort_ecs::{
    self as bevy_ecs,
    entity::Entity,
    event::EventReader,
    system::{Commands, Query, Res, ResMut, Resource},
};
use tort_input::{keyboard::KeyCode, mouse::MouseMotion, Input};
use tort_math::{Mat4, Vec2, Vec3};
use tort_time::Time;
use tort_window::{PrimaryWindow, Window};

use crate::Extract;

#[derive(Resource)]
pub struct Camera {
    camera_rig: CameraRig<LeftHanded>,
    field_of_view: f32,
    window_size: Vec2,
    near_plane: f32,
    far_plane: f32,

    sensitivity: Vec2,
    speed: f32,

    projection_matrix: Mat4,
    view_matrix: Mat4,
    view_projection_matrix: Mat4,
}

impl Camera {
    pub fn new(
        position: Vec3,
        field_of_view: f32,
        window_size: Vec2,
        near_plane: f32,
        far_plane: f32,
        sensitivity: Vec2,
        speed: f32,
    ) -> Self {
        let camera_rig = CameraRig::<LeftHanded>::builder()
            .with(Position::new(position))
            .with(YawPitch::new())
            .with(Smooth::new_position_rotation(0., 0.))
            .build();

        Self {
            camera_rig,
            field_of_view: field_of_view.to_radians(),
            window_size,
            near_plane,
            far_plane,
            sensitivity,
            speed,

            projection_matrix: Mat4::default(),
            view_matrix: Mat4::default(),
            view_projection_matrix: Mat4::default(),
        }
    }

    pub fn update(&mut self) {
        let final_transform = &self.camera_rig.final_transform;

        self.projection_matrix = Mat4::perspective_lh(
            self.field_of_view,
            self.window_size.x / self.window_size.y,
            self.near_plane,
            self.far_plane,
        );
        self.view_matrix = Mat4::look_at_lh(
            final_transform.position,
            final_transform.position + final_transform.forward(),
            final_transform.up(),
        );

        self.view_projection_matrix = self.projection_matrix * self.view_matrix;
    }

    #[inline]
    pub fn rig(&self) -> &CameraRig<LeftHanded> {
        &self.camera_rig
    }

    #[inline]
    pub fn rig_mut(&mut self) -> &mut CameraRig<LeftHanded> {
        &mut self.camera_rig
    }

    #[inline]
    pub fn field_of_view(&self) -> f32 {
        self.field_of_view.to_degrees()
    }

    #[inline]
    pub fn set_field_of_view(&mut self, field_of_view: f32) {
        self.field_of_view = field_of_view.to_radians();
    }

    #[inline]
    pub fn set_window_size(&mut self, window_size: Vec2) {
        self.window_size = window_size;
    }

    #[inline]
    pub fn near_plane(&self) -> f32 {
        self.near_plane
    }

    #[inline]
    pub fn set_near_plane(&mut self, near_plane: f32) {
        self.near_plane = near_plane;
    }

    #[inline]
    pub fn far_plane(&self) -> f32 {
        self.far_plane
    }

    #[inline]
    pub fn set_far_plane(&mut self, far_plane: f32) {
        self.far_plane = far_plane;
    }

    #[inline]
    pub fn sensitivity(&self) -> Vec2 {
        self.sensitivity
    }

    #[inline]
    pub fn set_sensitivity(&mut self, sensitivity: Vec2) {
        self.sensitivity = sensitivity;
    }

    #[inline]
    pub fn speed(&self) -> f32 {
        self.speed
    }

    #[inline]
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    #[inline]
    pub fn projection_matrix(&self) -> &Mat4 {
        &self.projection_matrix
    }

    #[inline]
    pub fn view_matrix(&self) -> &Mat4 {
        &self.view_matrix
    }

    #[inline]
    pub fn view_projection_matrix(&self) -> &Mat4 {
        &self.view_projection_matrix
    }
}

pub fn update_camera_system(
    mut camera: ResMut<Camera>,
    window: Query<(Entity, &Window, &PrimaryWindow)>,
    mut mouse_motion: EventReader<MouseMotion>,
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    let window_size = &window.single().1.resolution;

    camera.set_window_size(Vec2::new(window_size.width(), window_size.height()));

    let mut delta_pos = Vec3::ZERO;
    if keys.pressed(KeyCode::W) {
        delta_pos += Vec3::new(0.0, 0.0, 1.0);
    }

    if keys.pressed(KeyCode::A) {
        delta_pos += Vec3::new(-1.0, 0.0, 0.0);
    }

    if keys.pressed(KeyCode::S) {
        delta_pos += Vec3::new(0.0, 0.0, -1.0);
    }

    if keys.pressed(KeyCode::D) {
        delta_pos += Vec3::new(1.0, 0.0, 0.0);
    }

    delta_pos = camera.rig().final_transform.rotation * delta_pos * 2.0;

    if keys.pressed(KeyCode::Space) {
        delta_pos += Vec3::new(0.0, -1.0, 0.0);
    }

    if keys.pressed(KeyCode::LShift) {
        delta_pos += Vec3::new(0.0, 1.0, 0.0);
    }

    let speed = camera.speed();
    let sensitivity = camera.sensitivity();

    camera
        .rig_mut()
        .driver_mut::<Position>()
        .translate(delta_pos * time.delta_seconds() * speed);

    for event in mouse_motion.iter() {
        camera.rig_mut().driver_mut::<YawPitch>().rotate_yaw_pitch(
            event.delta.x * sensitivity.x,
            -event.delta.y * sensitivity.y,
        );
    }

    camera.rig_mut().update(time.delta_seconds());
    camera.update();
}

#[derive(Resource)]
pub struct ExtractedCamera {
    pub view_projection_matrix: Mat4,
}

impl From<&Camera> for ExtractedCamera {
    #[inline]
    fn from(camera: &Camera) -> Self {
        Self {
            view_projection_matrix: *camera.view_projection_matrix(),
        }
    }
}

pub fn extract_camera_system(mut commands: Commands, camera: Extract<Res<Camera>>) {
    commands.insert_resource(ExtractedCamera::from(&**camera))
}
