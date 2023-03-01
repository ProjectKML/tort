#![feature(c_variadic)]

pub mod backend;

mod extract_param;
pub mod pipelined_rendering;
pub mod renderer;
pub mod view;

use std::ops::{Deref, DerefMut};

pub use extract_param::*;
use tort_app::{self as bevy_app, App, AppLabel, CoreSchedule, Plugin, SubApp};
use tort_asset::{AddAsset, AssetServer};
use tort_ecs::{
    self as bevy_ecs,
    schedule::{
        apply_system_buffers, IntoSystemConfig, IntoSystemSetConfig, Schedule, ScheduleLabel,
        Schedules, SystemSet,
    },
    system::Resource,
    world::{Mut, World},
};
use tort_math::{Vec2, Vec3};

use crate::{
    backend::resource::pipeline::{PipelineCache, Shader, ShaderLoader},
    renderer::{render_system, BuiltinPipelines, FrameCtx},
    view::{extract_camera_system, update_camera_system, Camera, WindowRenderPlugin},
};

#[derive(Default)]
pub struct RenderPlugin;

/// The labels of the default App rendering sets.
///
/// The sets run in the order listed, with [`apply_system_buffers`] inserted between each set.
///
/// The `*Flush` sets are assigned to the copy of [`apply_system_buffers`]
/// that runs immediately after the matching system set.
/// These can be useful for ordering, but you almost never want to add your systems to these sets.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum RenderSet {
    /// The copy of [`apply_system_buffers`] that runs at the begining of this schedule.
    /// This is used for applying the commands from the [`ExtractSchedule`]
    ExtractCommands,
    /// Prepare render resources from the extracted data for the GPU.
    Prepare,
    /// The copy of [`apply_system_buffers`] that runs immediately after `Prepare`.
    PrepareFlush,
    /// Actual rendering happens here.
    /// In most cases, only the render backend should insert resources here.
    Render,
    /// The copy of [`apply_system_buffers`] that runs immediately after `Render`.
    RenderFlush,
    /// Cleanup render resources here.
    Cleanup,
    /// The copy of [`apply_system_buffers`] that runs immediately after `Cleanup`.
    CleanupFlush,
}

impl RenderSet {
    /// Sets up the base structure of the rendering [`Schedule`].
    ///
    /// The sets defined in this enum are configured to run in order,
    /// and a copy of [`apply_system_buffers`] is inserted into each `*Flush` set.
    pub fn base_schedule() -> Schedule {
        use RenderSet::*;

        let mut schedule = Schedule::new();

        // Create "stage-like" structure using buffer flushes + ordering
        schedule.add_system(apply_system_buffers.in_set(ExtractCommands));
        schedule.add_system(apply_system_buffers.in_set(PrepareFlush));
        schedule.add_system(apply_system_buffers.in_set(RenderFlush));
        schedule.add_system(apply_system_buffers.in_set(CleanupFlush));

        schedule.configure_set(ExtractCommands.before(Prepare));
        schedule.configure_set(Prepare.after(ExtractCommands).before(PrepareFlush));
        schedule.configure_set(Render.after(PrepareFlush).before(RenderFlush));
        schedule.configure_set(Cleanup.after(RenderFlush).before(CleanupFlush));

        schedule
    }
}

/// Schedule which extract data from the main world and inserts it into the render world.
///
/// This step should be kept as short as possible to increase the "pipelining potential" for
/// running the next frame while rendering the current frame.
///
/// This schedule is run on the main world, but its buffers are not applied
/// via [`Schedule::apply_system_buffers`](bevy_ecs::schedule::Schedule) until it is returned to the render world.
#[derive(ScheduleLabel, PartialEq, Eq, Debug, Clone, Hash)]
pub struct ExtractSchedule;

/// The simulation [`World`] of the application, stored as a resource.
/// This resource is only available during [`ExtractSchedule`] and not
/// during command application of that schedule.
/// See [`Extract`] for more details.
#[derive(Resource, Default)]
pub struct MainWorld(World);

impl Deref for MainWorld {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MainWorld {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A Label for the rendering sub-app.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
pub struct RenderApp;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<Shader>()
            .add_debug_asset::<Shader>()
            .init_asset_loader::<ShaderLoader>()
            .init_debug_asset_loader::<ShaderLoader>();

        let (instance, device) = renderer::init();

        app.insert_resource(instance.clone())
            .insert_resource(device.clone())
            .init_resource::<ScratchMainWorld>()
            .insert_resource(Camera::new(
                Vec3::ZERO,
                90.,
                Vec2::new(1600., 900.),
                0.1,
                1000.,
                Vec2::ONE,
                1.,
            ))
            .add_system(update_camera_system);

        let mut pipeline_cache = PipelineCache::new(device.clone());
        let asset_server = app.world.resource::<AssetServer>().clone();

        let builtin_pipelines = BuiltinPipelines::new(&asset_server, &mut pipeline_cache);

        let mut render_app = App::empty();
        render_app.add_simple_outer_schedule();
        let mut render_schedule = RenderSet::base_schedule();

        // Prepare the schedule which extracts data from the main world to the render world
        render_app.edit_schedule(ExtractSchedule, |schedule| {
            schedule
                .set_apply_final_buffers(false)
                .add_system(PipelineCache::extract_shaders_system)
                .add_system(extract_camera_system);
        });

        // This set applies the commands from the extract stage while the render schedule
        // is running in parallel with the main app.
        render_schedule.add_system(apply_extract_commands.in_set(RenderSet::ExtractCommands));

        render_schedule.add_system(
            PipelineCache::process_pipelines_system
                .before(render_system)
                .in_set(RenderSet::Render),
        );

        render_schedule.add_system(render_system.in_set(RenderSet::Render));
        render_schedule.add_system(World::clear_entities.in_set(RenderSet::Cleanup));

        let frame_ctx = FrameCtx::new(device.clone(), 2);

        render_app
            .add_schedule(CoreSchedule::Main, render_schedule)
            .insert_resource(instance)
            .insert_resource(device)
            .insert_resource(frame_ctx)
            .insert_resource(pipeline_cache)
            .insert_resource(builtin_pipelines)
            .insert_resource(asset_server);

        let (sender, receiver) = tort_time::create_time_channels();
        app.insert_resource(receiver);
        render_app.insert_resource(sender);

        app.insert_sub_app(RenderApp, SubApp::new(render_app, move |main_world, render_app| {
            #[cfg(feature = "trace")]
                let _render_span = info_span!("extract main app to render subapp").entered();
            {
                #[cfg(feature = "trace")]
                    let _stage_span =
                    info_span!("reserve_and_flush")
                        .entered();

                // reserve all existing main world entities for use in render_app
                // they can only be spawned using `get_or_spawn()`
                let total_count = main_world.entities().total_count();

                assert_eq!(
                    render_app.world.entities().len(),
                    0,
                    "An entity was spawned after the entity list was cleared last frame and before the extract schedule began. This is not supported",
                );

                // This is safe given the clear_entities call in the past frame and the assert above
                unsafe {
                    render_app
                        .world
                        .entities_mut()
                        .flush_and_reserve_invalid_assuming_no_entities(total_count);
                }
            }

            // run extract schedule
            extract(main_world, render_app);
        }));

        app.add_plugin(WindowRenderPlugin);
    }
}

/// A "scratch" world used to avoid allocating new worlds every frame when
/// swapping out the [`MainWorld`] for [`ExtractSchedule`].
#[derive(Resource, Default)]
struct ScratchMainWorld(World);

/// Executes the [`ExtractSchedule`] step of the renderer.
/// This updates the render world with the extracted ECS data of the current frame.
fn extract(main_world: &mut World, render_app: &mut App) {
    // temporarily add the app world to the render world as a resource
    let scratch_world = main_world.remove_resource::<ScratchMainWorld>().unwrap();
    let inserted_world = std::mem::replace(main_world, scratch_world.0);
    render_app.world.insert_resource(MainWorld(inserted_world));

    render_app.world.run_schedule(ExtractSchedule);

    // move the app world back, as if nothing happened.
    let inserted_world = render_app.world.remove_resource::<MainWorld>().unwrap();
    let scratch_world = std::mem::replace(main_world, inserted_world.0);
    main_world.insert_resource(ScratchMainWorld(scratch_world));
}

/// Applies the commands from the extract schedule. This happens during
/// the render schedule rather than during extraction to allow the commands to run in parallel with the
/// main app when pipelined rendering is enabled.
fn apply_extract_commands(render_world: &mut World) {
    render_world.resource_scope(|render_world, mut schedules: Mut<Schedules>| {
        schedules
            .get_mut(&ExtractSchedule)
            .unwrap()
            .apply_system_buffers(render_world);
    });
}
