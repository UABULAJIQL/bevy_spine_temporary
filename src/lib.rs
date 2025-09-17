//! A Bevy plugin for Spine 3.8
//!
//! Add [`SpinePlugin`] to your Bevy app and spawn a [`SpineBundle`] to get started!

use std::collections::VecDeque;
use std::mem;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;

use bevy::mesh::{Indices, MeshVertexAttribute};
use bevy::platform::collections::HashMap;
use bevy::render::render_resource::{PrimitiveTopology, VertexFormat};
use spine::controller::SkeletonControllerSettings;
use spine::draw::CullDirection;
use spine::{AnimationEvent, Skeleton};
use spine::{AnimationStateData, BoneHandle};

use assets::{AtlasLoader, SkeletonJsonLoader};
use materials::DARK_COLOR_ATTRIBUTE;
use materials::SpineMaterialInfo;
use textures::{SpineTexture, SpineTextureCreateEvent, SpineTextureDisposeEvent, SpineTextures};

#[cfg(feature = "pma_fix")]
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
#[cfg(feature = "pma_fix")]
use spine::atlas::{AtlasFilter, AtlasWrap};
#[cfg(feature = "pma_fix")]
use textures::SpineTextureConfig;

#[cfg(feature = "default_shader")]
use bevy::asset::load_internal_binary_asset;

#[cfg(any(
    feature = "additive_material",
    feature = "additive_pma_material",
    feature = "multiply_material",
    feature = "multiply_pma_material",
    feature = "normal_material",
    feature = "normal_pma_material",
    feature = "screen_material",
    feature = "screen_pma_material",
))]
use bevy::sprite_render::Material2dPlugin;

#[cfg(feature = "default_shader")]
use materials::SHADER_HANDLE;

#[cfg(any(
    feature = "default_3d_material",
    feature = "additive_material",
    feature = "additive_pma_material",
    feature = "multiply_material",
    feature = "multiply_pma_material",
    feature = "normal_material",
    feature = "normal_pma_material",
    feature = "screen_material",
    feature = "screen_pma_material",
))]
use materials::SpineMaterialPlugin;

#[cfg(feature = "additive_material")]
use materials::material_2d::SpineAdditiveMaterial;
#[cfg(feature = "additive_pma_material")]
use materials::material_2d::SpineAdditivePmaMaterial;
#[cfg(feature = "multiply_material")]
use materials::material_2d::SpineMultiplyMaterial;
#[cfg(feature = "multiply_pma_material")]
use materials::material_2d::SpineMultiplyPmaMaterial;
#[cfg(feature = "normal_material")]
use materials::material_2d::SpineNormalMaterial;
#[cfg(feature = "normal_pma_material")]
use materials::material_2d::SpineNormalPmaMaterial;
#[cfg(feature = "screen_material")]
use materials::material_2d::SpineScreenMaterial;
#[cfg(feature = "screen_pma_material")]
use materials::material_2d::SpineScreenPmaMaterial;

#[cfg(feature = "default_3d_material")]
use materials::material_3d::Spine3DMaterial;

pub use crate::assets::*;
pub use crate::crossfades::Crossfades;
pub use crate::entity_sync::*;
pub use crate::handle::*;
pub use crate::spine::Color;

/// See [`spine`] docs for more info.
pub use spine::controller::SkeletonController;

pub use spine;

/// System sets for Spine systems.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, SystemSet)]
pub enum SpineSystem {
    /// Loads [`SkeletonData`] assets which must exist before a [`SpineBundle`] can fully load.
    Load,
    /// Spawns helper entities associated with a [`SpineBundle`] for drawing meshes and
    /// (optionally) adding bone entities (see [`SpineLoader`]).
    Spawn,
    /// An [`apply_deferred`] to load the spine helper entities this frame.
    SpawnFlush,
    /// Sends [`SpineReadyEvent`] after [`SpineSystem::SpawnFlush`], indicating [`Spine`] components
    /// on newly spawned [`SpineBundle`]s can now be interacted with.
    Ready,
    /// Advances all animations and processes Spine events (see [`SpineEvent`]).
    UpdateAnimation,
    /// Updates all Spine meshes.
    UpdateMeshes,
    /// Updates all Spine materials.
    UpdateMaterials,
    #[cfg(feature = "pma_fix")]
    /// Adjusts Spine textures to render properly.
    AdjustSpineTextures,
}

/// Helper sets for interacting with Spine systems.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, SystemSet)]
pub enum SpineSet {
    /// A helper Set occuring after [`SpineSystem::Ready`] but before Spine update systems, so that
    /// systems can configure a newly spawned skeleton before they are updated for the first time.
    OnReady,
    /// A helper Set occuring after [`SpineSystem::UpdateAnimation`] but before
    /// [`SpineSystem::UpdateMeshes`], so that systems can handle events immediately after the
    /// skeleton updates but before it renders.
    OnEvent,
    /// A helper set occuring simultaneously with [`SpineSystem::UpdateMeshes`], useful for custom
    /// mesh creation when using [`SpineDrawer::None`].
    OnUpdateMesh,
}

/// Add Spine support to Bevy!
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_spine::SpinePlugin;
/// # fn doc() {
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(SpinePlugin)
///     // ...
///     .run();
/// # }
/// ```
pub struct SpinePlugin;

impl Plugin for SpinePlugin {
    fn build(&self, app: &mut App) {
        #[cfg(any(
            feature = "additive_material",
            feature = "additive_pma_material",
            feature = "multiply_material",
            feature = "multiply_pma_material",
            feature = "normal_material",
            feature = "normal_pma_material",
            feature = "screen_material",
            feature = "screen_pma_material",
        ))]
        app.add_plugins((
            #[cfg(feature = "normal_material")]
            Material2dPlugin::<SpineNormalMaterial>::default(),
            #[cfg(feature = "additive_material")]
            Material2dPlugin::<SpineAdditiveMaterial>::default(),
            #[cfg(feature = "multiply_material")]
            Material2dPlugin::<SpineMultiplyMaterial>::default(),
            #[cfg(feature = "screen_material")]
            Material2dPlugin::<SpineScreenMaterial>::default(),
            #[cfg(feature = "normal_pma_material")]
            Material2dPlugin::<SpineNormalPmaMaterial>::default(),
            #[cfg(feature = "additive_pma_material")]
            Material2dPlugin::<SpineAdditivePmaMaterial>::default(),
            #[cfg(feature = "multiply_pma_material")]
            Material2dPlugin::<SpineMultiplyPmaMaterial>::default(),
            #[cfg(feature = "screen_pma_material")]
            Material2dPlugin::<SpineScreenPmaMaterial>::default(),
        ))
        .add_plugins((
            #[cfg(feature = "normal_material")]
            SpineMaterialPlugin::<SpineNormalMaterial>::default(),
            #[cfg(feature = "additive_material")]
            SpineMaterialPlugin::<SpineAdditiveMaterial>::default(),
            #[cfg(feature = "multiply_material")]
            SpineMaterialPlugin::<SpineMultiplyMaterial>::default(),
            #[cfg(feature = "screen_material")]
            SpineMaterialPlugin::<SpineScreenMaterial>::default(),
            #[cfg(feature = "normal_pma_material")]
            SpineMaterialPlugin::<SpineNormalPmaMaterial>::default(),
            #[cfg(feature = "additive_pma_material")]
            SpineMaterialPlugin::<SpineAdditivePmaMaterial>::default(),
            #[cfg(feature = "multiply_pma_material")]
            SpineMaterialPlugin::<SpineMultiplyPmaMaterial>::default(),
            #[cfg(feature = "screen_pma_material")]
            SpineMaterialPlugin::<SpineScreenPmaMaterial>::default(),
        ));

        #[cfg(feature = "default_3d_material")]
        app.add_plugins(SpineMaterialPlugin::<Spine3DMaterial>::default());

        app.add_plugins(SpineSyncPlugin::first())
            .init_resource::<SpineEventQueue>()
            .insert_resource(SpineTextures::init())
            .insert_resource(SpineReadyEvents::default())
            .add_message::<SpineTextureCreateEvent>()
            .add_message::<SpineTextureDisposeEvent>()
            .init_asset::<Atlas>()
            .init_asset::<SkeletonJson>()
            .init_asset::<SkeletonBinary>()
            .init_asset::<SkeletonData>()
            .init_asset_loader::<AtlasLoader>()
            .init_asset_loader::<SkeletonJsonLoader>()
            .init_asset_loader::<SkeletonBinaryLoader>()
            .add_message::<SpineReadyEvent>()
            .add_message::<SpineEvent>()
            .add_systems(
                Update,
                (
                    spine_load.in_set(SpineSystem::Load),
                    spine_spawn
                        .in_set(SpineSystem::Spawn)
                        .after(SpineSystem::Load),
                    spine_ready
                        .in_set(SpineSystem::Ready)
                        .after(SpineSystem::Spawn)
                        .before(SpineSet::OnReady),
                    spine_update_animation
                        .in_set(SpineSystem::UpdateAnimation)
                        .after(SpineSet::OnReady)
                        .before(SpineSet::OnEvent),
                    spine_get_renderables
                        .pipe(spine_update_meshes)
                        .in_set(SpineSystem::UpdateMeshes)
                        .in_set(SpineSet::OnUpdateMesh)
                        .after(SpineSystem::UpdateAnimation)
                        .after(SpineSet::OnEvent),
                    ApplyDeferred
                        .in_set(SpineSystem::SpawnFlush)
                        .after(SpineSystem::Spawn)
                        .before(SpineSystem::Ready),
                ),
            );

        #[cfg(feature = "pma_fix")]
        app.add_systems(
            PostUpdate,
            adjust_spine_textures.in_set(SpineSystem::AdjustSpineTextures),
        );

        #[cfg(feature = "default_shader")]
        load_internal_binary_asset!(app, SHADER_HANDLE, "spine.wgsl", |bytes, path: String| {
            Shader::from_wgsl(String::from_utf8_lossy(bytes), path)
        });
    }
}

#[derive(Resource, Default)]
struct SpineEventQueue(Arc<Mutex<VecDeque<SpineEvent>>>);

/// A live Spine [`SkeletonController`] [`Component`], ready to be manipulated.
///
/// This component does not exist on [`SpineBundle`] initially, since Spine assets may not yet be
/// loaded when an entity is spawned. Querying for this component type guarantees that all entities
/// containing it have a Spine rig that is ready to use.
#[derive(Component, Debug)]
pub struct Spine(pub SkeletonController);

/// When loaded, a [`Spine`] entity has children entities attached to it, each containing this
/// component.
///
/// To disable creation of these child entities, see [`SpineLoader::without_children`].
///
/// The bones are not automatically synchronized, but can be synchronized easily by adding a
/// [`SpineSync`] component.
#[derive(Component, Debug)]
pub struct SpineBone {
    pub spine_entity: Entity,
    pub handle: BoneHandle,
    pub name: String,
    pub parent: Option<SpineBoneParent>,
}

#[derive(Debug)]
pub struct SpineBoneParent {
    pub entity: Entity,
    pub handle: BoneHandle,
}

#[derive(Component, Clone)]
pub struct SpineMeshes;

/// Marker component for child entities containing [`Mesh`] components for Spine rendering.
///
/// By default, the meshes may contain several meshes all combined into one to reduce draw calls
/// and improve performance. To interact with individual Spine meshes, see
/// [`SpineSettings::drawer`].
#[derive(Component, Debug, Clone)]
pub struct SpineMesh {
    pub spine_entity: Entity,
    pub handle: Handle<Mesh>,
    pub state: SpineMeshState,
}

/// The state of this [`SpineMesh`].
#[derive(Default, Component, Debug, Clone)]
pub enum SpineMeshState {
    /// This Spine mesh contains no mesh data and should not render.
    #[default]
    Empty,
    /// This Spine mesh contains mesh data and should render.
    Renderable { info: SpineMaterialInfo },
}

impl core::ops::Deref for Spine {
    type Target = SkeletonController;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::ops::DerefMut for Spine {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// The async loader for Spine assets. Waits for Spine assets to be ready in the [`AssetServer`],
/// then initializes child entities, and finally attaches the live [`Spine`] component.
///
/// When spawning a [`SpineLoader`] (typically through [`SpineBundle`]), it will create child
/// entities representing the bones of a skeleton (see [`SpineBone`]). These bones are not
/// synchronized (see [`SpineSync`]), and can be disabled entirely using
/// [`SpineLoader::without_children`].
#[derive(Component, Debug)]
pub enum SpineLoader {
    /// The spine rig is still loading.
    Loading {
        /// If true, will spawn child entities for each bone in the skeleton (see [`SpineBone`]).
        with_children: bool,
    },
    /// The spine rig is ready.
    Ready,
    /// The spine rig failed to load.
    Failed,
}

impl Default for SpineLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl SpineLoader {
    pub fn new() -> Self {
        Self::with_children()
    }

    pub fn with_children() -> Self {
        Self::Loading {
            with_children: true,
        }
    }

    /// Load a [`Spine`] entity without child entities containing [`SpineBone`] components.
    ///
    /// Renderable mesh child entities are still created.
    ///
    /// ```
    /// # use bevy::prelude::*;
    /// # use bevy_spine::{SpineLoader, SpineBundle};
    /// # fn doc(mut commands: Commands) {
    /// commands.spawn(SpineBundle {
    ///     // ..
    ///     loader: SpineLoader::without_children(),
    ///     ..default()
    /// });
    /// # }
    /// ```
    pub fn without_children() -> Self {
        Self::Loading {
            with_children: false,
        }
    }
}

/// Settings for how this Spine updates and renders.
///
/// Typically set in [`SpineBundle`] when spawning an entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpineSettings {
    /// Indicates if default Spine materials should be used (default: `true`).
    ///
    /// If `false`, a custom [`SpineMaterial`](`materials::SpineMaterial`) should be configured for
    /// this Spine.
    pub default_materials: bool,
    /// Indicates how the meshes should be drawn.
    #[cfg(all(feature = "2d", feature = "3d"))]
    pub mesh_type: SpineMeshType,
    /// The drawer this Spine should use to create its meshes.
    pub drawer: SpineDrawer,
}

/// Mesh types to use in [`SpineSettings`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(all(feature = "2d", feature = "3d"))]
pub enum SpineMeshType {
    /// Render meshes in 2D.
    Mesh2D,
    /// Render meshes in 3D. Requires a custom [`SpineMaterial`](`materials::SpineMaterial`) since
    /// the default materials do not support 3D meshes.
    Mesh3D,
}

/// Drawer methods to use in [`SpineSettings`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpineDrawer {
    /// Draw each slot as a separate mesh, each represented by one [`SpineMesh`].
    ///
    /// Useful if individual meshes need separate materials, z-depth, or other rendering
    /// differences. Less performant, but more versatile than [`SpineDrawer::Combined`].
    Separated,
    /// Combine multiple slots into a single mesh.
    ///
    /// The default, and most performanent drawer method. Suitable for most use cases.
    Combined,
    /// Do not update meshes at all.
    None,
}

impl Default for SpineSettings {
    fn default() -> Self {
        Self {
            default_materials: true,
            #[cfg(all(feature = "2d", feature = "3d"))]
            mesh_type: SpineMeshType::Mesh2D,
            drawer: SpineDrawer::Combined,
        }
    }
}

/// Bundle for Spine skeletons with all the necessary components.
///
/// See [`SkeletonData::new_from_json`] or [`SkeletonData::new_from_binary`] for example usages.
///
/// Note that this bundle does not contain the [`Spine`] component itself, which is the primary way
/// to query and interact with Spine skeletons. Instead, a [`SpineLoader`] is added which ensures
/// that all the necessary assets ([`Atlas`] and [`SkeletonJson`]/[`SkeletonBinary`]) are loaded
/// before instantiating the Spine skeleton. This ensures that querying for [`Spine`] components
/// will always yield fully instantiated skeletons.
///
/// It is possible to spawn a Spine skeleton and initialize it in the same frame. To do so, ensure
/// that the spawning system occurs before [`SpineSystem::Spawn`] and the initializing system is in
/// the [`SpineSet::OnReady`] set (assuming the [`SkeletonData`] has already been loaded). Listen
/// for [`SpineReadyEvent`] to get newly loaded skeletons.
///
/// ```
/// use bevy::prelude::*;
/// use bevy_spine::prelude::*;
///
/// # let mut app = App::new();
/// {
///     // in main() or a plugin
///     app.add_systems(
///         Update,
///         (
///             spawn_spine.before(SpineSystem::Spawn),
///             init_spine.in_set(SpineSet::OnReady),
///         ),
///     );
/// }
///
/// #[derive(Resource)]
/// struct MyGameAssets {
///     // loaded ahead of time
///     skeleton: Handle<SkeletonData>
/// }
///
/// #[derive(Component)]
/// struct MySpine;
///
/// fn spawn_spine(
///     mut commands: Commands,
///     my_game_assets: Res<MyGameAssets>
/// ) {
///     commands.spawn((
///         SpineBundle {
///             skeleton: SkeletonDataHandle(my_game_assets.skeleton.clone()),
///             ..default()
///         },
///         MySpine
///     ));
/// }
///
/// fn init_spine(
///     mut spine_ready_events: EventReader<SpineReadyEvent>,
///     mut spine_query: Query<&mut Spine, With<MySpine>>
/// ) {
///     for spine_ready_event in spine_ready_events.read() {
///         if let Ok(mut spine) = spine_query.get_mut(spine_ready_event.entity) {
///             // the skeleton will start playing the animation the same frame it spawns on
///             spine.animation_state.set_animation_by_name(0, "animation", true);
///         }
///     }
/// }
/// ```
#[derive(Default, Bundle)]
pub struct SpineBundle {
    pub loader: SpineLoader,
    pub settings: SpineSettings,
    pub skeleton: SkeletonDataHandle,
    pub crossfades: Crossfades,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
}

/// An [`Event`] which is sent once a [`SpineLoader`] has fully loaded a skeleton and attached the
/// [`Spine`] component.
///
/// For convenience, systems receiving this event can be added to the [`SpineSet::OnReady`] set to
/// receive this after events are sent, but before the first [`SkeletonController`] update.
#[derive(Debug, Clone, Message)]
pub struct SpineReadyEvent {
    /// The entity containing the [`Spine`] component.
    pub entity: Entity,
    /// A list of all bones (if spawned, see [`SpineBone`]).
    pub bones: HashMap<String, Entity>,
}

/// A Spine event fired from a playing animation.
///
/// Sent in [`SpineSystem::UpdateAnimation`].
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_spine::prelude::*;
/// // bevy system
/// fn on_spine_event(
///     mut spine_events: EventReader<SpineEvent>,
///     mut commands: Commands,
///     asset_server: Res<AssetServer>,
/// ) {
///     for event in spine_events.read() {
///         if let SpineEvent::Event { name, entity, .. } = event {
///             println!("spine event fired: {name}");
///             println!("from entity: {entity:?}");
///         }
///     }
/// }
/// ```
#[derive(Debug, Clone, Message)]
pub enum SpineEvent {
    Start {
        entity: Entity,
        animation: String,
    },
    Interrupt {
        entity: Entity,
        animation: String,
    },
    End {
        entity: Entity,
        animation: String,
    },
    Complete {
        entity: Entity,
        animation: String,
    },
    Dispose {
        entity: Entity,
    },
    Event {
        entity: Entity,
        name: String,
        int: i32,
        float: f32,
        string: String,
        audio_path: String,
        volume: f32,
        balance: f32,
    },
}

/// Queued ready events, to be sent after [`SpineSystem::SpawnFlush`].
#[derive(Default, Resource)]
struct SpineReadyEvents(Vec<SpineReadyEvent>);

#[allow(clippy::too_many_arguments)]
fn spine_load(
    mut skeleton_data_assets: ResMut<Assets<SkeletonData>>,
    mut texture_create_events: MessageWriter<SpineTextureCreateEvent>,
    mut texture_dispose_events: MessageWriter<SpineTextureDisposeEvent>,
    atlases: Res<Assets<Atlas>>,
    jsons: Res<Assets<SkeletonJson>>,
    binaries: Res<Assets<SkeletonBinary>>,
    spine_textures: Res<SpineTextures>,
    asset_server: Res<AssetServer>,
) {
    // check if any assets are loading, else, early out to avoid triggering change detection
    let mut loading = false;

    for (_, skeleton_data_asset) in skeleton_data_assets.iter() {
        if matches!(skeleton_data_asset.status, SkeletonDataStatus::Loading) {
            loading = true;
            break;
        }
    }

    if loading {
        for (_, skeleton_data_asset) in skeleton_data_assets.iter_mut() {
            let SkeletonData {
                atlas_handle,
                kind,
                status,
                premultiplied_alpha,
            } = skeleton_data_asset;

            if matches!(status, SkeletonDataStatus::Loading) {
                let Some(atlas) = atlases.get(atlas_handle) else { continue };

                *premultiplied_alpha = atlas.premultiplied_alpha;

                match kind {
                    SkeletonDataKind::JsonFile(json_handle) => {
                        let Some(json) = jsons.get(json_handle) else { continue };
                        let skeleton_json = spine::SkeletonJson::new(atlas.atlas.clone());

                        match skeleton_json.read_skeleton_data(&json.json) {
                            Ok(skeleton_data) => {
                                *status = SkeletonDataStatus::Loaded(Arc::new(skeleton_data));
                            }
                            Err(_error) => {
                                #[cfg(feature = "debug")]
                                warn!("{_error:?}");

                                *status = SkeletonDataStatus::Failed;
                                continue;
                            }
                        }
                    }
                    SkeletonDataKind::BinaryFile(binary_handle) => {
                        let Some(binary) = binaries.get(binary_handle) else { continue };
                        let skeleton_binary = spine::SkeletonBinary::new(atlas.atlas.clone());
                        match skeleton_binary.read_skeleton_data(&binary.binary) {
                            Ok(skeleton_data) => {
                                *status = SkeletonDataStatus::Loaded(Arc::new(skeleton_data));
                            }
                            Err(_error) => {
                                #[cfg(feature = "debug")]
                                warn!("{_error:?}");

                                *status = SkeletonDataStatus::Failed;
                                continue;
                            }
                        }
                    }
                }
            }
        }
    }

    spine_textures.update(
        asset_server.as_ref(),
        atlases.as_ref(),
        &mut texture_create_events,
        &mut texture_dispose_events,
    );
}

#[allow(clippy::too_many_arguments)]
fn spine_spawn(
    mut skeleton_query: Query<(
        &mut SpineLoader,
        Entity,
        &SkeletonDataHandle,
        Option<&Crossfades>,
    )>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut ready_events: ResMut<SpineReadyEvents>,
    mut skeleton_data_assets: ResMut<Assets<SkeletonData>>,
    spine_event_queue: Res<SpineEventQueue>,
) {
    for (mut spine_loader, spine_entity, data_handle, crossfades) in skeleton_query.iter_mut() {
        if let SpineLoader::Loading { with_children } = spine_loader.as_ref() {
            let Some(skeleton_data_asset) = skeleton_data_assets.get_mut(&data_handle.0) else {
                continue;
            };

            match &skeleton_data_asset.status {
                SkeletonDataStatus::Loaded(skeleton_data) => {
                    let mut animation_state_data = AnimationStateData::new(skeleton_data.clone());

                    if let Some(crossfades) = crossfades {
                        crossfades.apply(&mut animation_state_data);
                    }

                    let mut controller = SkeletonController::new(
                        skeleton_data.clone(),
                        Arc::new(animation_state_data),
                    )
                    .with_settings(
                        SkeletonControllerSettings::new()
                            .with_cull_direction(CullDirection::CounterClockwise)
                            .with_premultiplied_alpha(skeleton_data_asset.premultiplied_alpha),
                    );

                    let events = spine_event_queue.0.clone();

                    controller
                        .animation_state
                        .set_listener(move |_, animation_event| match animation_event {
                            AnimationEvent::Start { track_entry } => {
                                let mut events = events.lock().unwrap();

                                events.push_back(SpineEvent::Start {
                                    entity: spine_entity,
                                    animation: track_entry.animation().name().to_owned(),
                                });
                            }
                            AnimationEvent::Interrupt { track_entry } => {
                                let mut events = events.lock().unwrap();

                                events.push_back(SpineEvent::Interrupt {
                                    entity: spine_entity,
                                    animation: track_entry.animation().name().to_owned(),
                                });
                            }
                            AnimationEvent::End { track_entry } => {
                                let mut events = events.lock().unwrap();

                                events.push_back(SpineEvent::End {
                                    entity: spine_entity,
                                    animation: track_entry.animation().name().to_owned(),
                                });
                            }
                            AnimationEvent::Complete { track_entry } => {
                                let mut events = events.lock().unwrap();

                                events.push_back(SpineEvent::Complete {
                                    entity: spine_entity,
                                    animation: track_entry.animation().name().to_owned(),
                                });
                            }
                            AnimationEvent::Dispose { .. } => {
                                let mut events = events.lock().unwrap();

                                events.push_back(SpineEvent::Dispose {
                                    entity: spine_entity,
                                });
                            }
                            AnimationEvent::Event {
                                name,
                                int,
                                float,
                                string,
                                audio_path,
                                volume,
                                balance,
                                ..
                            } => {
                                let mut events = events.lock().unwrap();

                                events.push_back(SpineEvent::Event {
                                    entity: spine_entity,
                                    name: name.to_owned(),
                                    int,
                                    float,
                                    string: string.to_owned(),
                                    audio_path: audio_path.to_owned(),
                                    volume,
                                    balance,
                                });
                            }
                        });

                    controller.skeleton.set_to_setup_pose();

                    let mut bones = HashMap::new();

                    if let Ok(mut entity_commands) = commands.get_entity(spine_entity) {
                        entity_commands
                            .with_children(|parent| {
                                parent
                                    .spawn((
                                        Name::new("spine_meshes"),
                                        SpineMeshes,
                                        Transform::default(),
                                        GlobalTransform::default(),
                                        Visibility::default(),
                                        InheritedVisibility::default(),
                                        ViewVisibility::default(),
                                    ))
                                    .with_children(|parent| {
                                        parent.spawn((
                                            Name::new("spine_mesh 0"),
                                            SpineMesh {
                                                spine_entity,
                                                handle: meshes.add(new_empty_mesh()),
                                                state: SpineMeshState::Empty,
                                            },
                                            Transform::default(),
                                            GlobalTransform::default(),
                                            Visibility::default(),
                                            InheritedVisibility::default(),
                                            ViewVisibility::default(),
                                        ));
                                    });

                                if *with_children {
                                    spawn_bones(
                                        spine_entity,
                                        None,
                                        parent,
                                        &controller.skeleton,
                                        controller.skeleton.bone_root().handle(),
                                        &mut bones,
                                    );
                                }
                            })
                            .insert(Spine(controller));
                    }

                    *spine_loader = SpineLoader::Ready;

                    ready_events.0.push(SpineReadyEvent {
                        entity: spine_entity,
                        bones,
                    });
                }
                SkeletonDataStatus::Loading => {}
                SkeletonDataStatus::Failed => {
                    *spine_loader = SpineLoader::Failed;
                }
            }
        }
    }
}

fn spawn_bones(
    spine_entity: Entity,
    bone_parent: Option<SpineBoneParent>,
    parent: &mut ChildSpawnerCommands,
    skeleton: &Skeleton,
    bone: BoneHandle,
    bones: &mut HashMap<String, Entity>,
) {
    if let Some(bone) = bone.get(skeleton) {
        let mut transform = Transform::default();

        transform.translation.x = bone.applied_x();
        transform.translation.y = bone.applied_y();
        transform.translation.z = 0.;

        transform.rotation = Quat::from_axis_angle(Vec3::Z, bone.applied_rotation().to_radians());

        transform.scale.x = bone.applied_scale_x();
        transform.scale.y = bone.applied_scale_y();

        let bone_entity = parent
            .spawn((
                Name::new(format!("spine_bone ({})", bone.data().name())),
                transform,
                GlobalTransform::default(),
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ))
            .insert(SpineBone {
                spine_entity,
                handle: bone.handle(),
                name: bone.data().name().to_owned(),
                parent: bone_parent,
            })
            .with_children(|parent| {
                for child in bone.children() {
                    spawn_bones(
                        spine_entity,
                        Some(SpineBoneParent {
                            entity: parent.target_entity(),
                            handle: bone.handle(),
                        }),
                        parent,
                        skeleton,
                        child.handle(),
                        bones,
                    );
                }
            })
            .id();

        bones.insert(bone.data().name().to_owned(), bone_entity);
    }
}

fn spine_ready(
    mut ready_events: ResMut<SpineReadyEvents>,
    mut ready_writer: MessageWriter<SpineReadyEvent>,
) {
    for event in mem::take(&mut ready_events.0) {
        ready_writer.write(event);
    }
}

fn spine_update_animation(
    mut spine_query: Query<(Entity, &mut Spine)>,
    mut spine_events: MessageWriter<SpineEvent>,
    time: Res<Time>,
    spine_event_queue: Res<SpineEventQueue>,
) {
    for (_, mut spine) in spine_query.iter_mut() {
        spine.update(time.delta_secs());
    }

    {
        let mut events = spine_event_queue.0.lock().unwrap();

        while let Some(event) = events.pop_front() {
            spine_events.write(event);
        }
    }
}

fn spine_get_renderables(
    meshes_query: Query<(Entity, &ChildOf, &Children), With<SpineMeshes>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut spine_query: Query<(&mut Spine, Option<&SpineSettings>)>,
) -> Vec<
    Option<
        Vec<(
            Option<usize>,
            Option<*const spine::c::c_void>,
            Vec<[f32; 2]>,
            Vec<u16>,
            Vec<[f32; 2]>,
            Vec<[f32; 4]>,
            Vec<[f32; 4]>,
            spine::BlendMode,
            bool,
        )>,
    >,
> {
    meshes_query
        .into_iter()
        .map(|(entity, parent, children)| {
            let spine_entity = parent.parent();
            let Ok((mut spine, settings)) = spine_query.get_mut(spine_entity) else { return None };
            let SpineSettings { drawer, .. } = settings.cloned().unwrap_or_default();

            let renderables = match drawer {
                SpineDrawer::Combined => spine
                    .0
                    .combined_renderables()
                    .into_iter()
                    .map(|mut renderable| {
                        (
                            None,
                            renderable.attachment_renderer_object,
                            mem::take(&mut renderable.vertices),
                            mem::take(&mut renderable.indices),
                            mem::take(&mut renderable.uvs),
                            mem::take(&mut renderable.colors),
                            mem::take(&mut renderable.dark_colors),
                            renderable.blend_mode,
                            renderable.premultiplied_alpha,
                        )
                    })
                    .collect::<Vec<_>>(),
                SpineDrawer::Separated => spine
                    .0
                    .renderables()
                    .into_iter()
                    .map(|mut renderable| {
                        let colors = vec![
                            [
                                renderable.color.r,
                                renderable.color.g,
                                renderable.color.b,
                                renderable.color.a
                            ];
                            renderable.vertices.len()
                        ];

                        let dark_colors = vec![
                            [
                                renderable.dark_color.r,
                                renderable.dark_color.g,
                                renderable.dark_color.b,
                                renderable.dark_color.a
                            ];
                            renderable.vertices.len()
                        ];

                        (
                            Some(renderable.slot_index),
                            renderable.attachment_renderer_object,
                            mem::take(&mut renderable.vertices),
                            mem::take(&mut renderable.indices),
                            mem::take(&mut renderable.uvs),
                            colors,
                            dark_colors,
                            renderable.blend_mode,
                            renderable.premultiplied_alpha,
                        )
                    })
                    .collect(),
                SpineDrawer::None => return None,
            };

            if children.len() < renderables.len() {
                let Ok(mut entity_commands) = commands.get_entity(entity) else { return None };

                entity_commands.with_children(|parent| {
                    (children.len()..renderables.len()).into_iter().fold(
                        0.001 * children.len() as f32,
                        |z, i| {
                            parent.spawn((
                                Name::new(format!("spine_mesh {i}")),
                                SpineMesh {
                                    spine_entity,
                                    handle: meshes.add(new_empty_mesh()),
                                    state: SpineMeshState::Empty,
                                },
                                Transform::from_xyz(0., 0., z),
                                GlobalTransform::default(),
                                Visibility::default(),
                                InheritedVisibility::default(),
                                ViewVisibility::default(),
                            ));

                            z + 0.001
                        },
                    );
                });
            }

            Some(renderables)
        })
        .collect()
}

#[allow(clippy::type_complexity)]
fn spine_update_meshes(
    input: In<
        Vec<
            Option<
                Vec<(
                    Option<usize>,
                    Option<*const spine::c::c_void>,
                    Vec<[f32; 2]>,
                    Vec<u16>,
                    Vec<[f32; 2]>,
                    Vec<[f32; 4]>,
                    Vec<[f32; 4]>,
                    spine::BlendMode,
                    bool,
                )>,
            >,
        >,
    >,
    asset_server: Res<AssetServer>,
    #[cfg(not(all(feature = "2d", feature = "3d")))] //
    meshes_query: Query<&Children, With<SpineMeshes>>,
    #[cfg(all(feature = "2d", feature = "3d"))] //
    meshes_query: Query<(&ChildOf, &Children), With<SpineMeshes>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    #[cfg(all(feature = "2d", feature = "3d"))] //
    mut spine_query: Query<Option<&SpineSettings>>,
    #[cfg(all(feature = "2d", not(feature = "3d")))] //
    mut mesh_query: Query<(Entity, &mut SpineMesh, Option<&Mesh2d>)>,
    #[cfg(all(feature = "3d", not(feature = "2d")))] //
    mut mesh_query: Query<(Entity, &mut SpineMesh, Option<&Mesh3d>)>,
    #[cfg(all(feature = "2d", feature = "3d"))] //
    mut mesh_query: Query<(Entity, &mut SpineMesh, Option<&Mesh2d>, Option<&Mesh3d>)>,
) {
    for (meshes_item, renderables) in meshes_query.iter().zip(input.0) {
        #[cfg(all(feature = "2d", feature = "3d"))]
        let Ok(settings) = spine_query.get_mut(meshes_item.0.parent()) else { continue };
        #[cfg(all(feature = "2d", feature = "3d"))]
        let SpineSettings { mesh_type, .. } = settings.cloned().unwrap_or_default();

        let Some(renderables) = renderables else { continue };

        #[cfg(not(all(feature = "2d", feature = "3d")))]
        let mut children = meshes_item.iter();
        #[cfg(all(feature = "2d", feature = "3d"))]
        let mut children = meshes_item.1.iter();

        for (
            slot_index,
            attachment_renderer_object,
            vertices,
            indices,
            uvs,
            colors,
            dark_colors,
            blend_mode,
            premultiplied_alpha,
        ) in renderables
        {
            let Some(child) = children.next() else { break };

            #[cfg(all(feature = "2d", not(feature = "3d")))]
            let Ok((
                spine_mesh_entity, //
                mut spine_mesh,
                spine_2d_mesh,
            )) = mesh_query.get_mut(child)
            else {
                continue;
            };

            #[cfg(all(feature = "3d", not(feature = "2d")))]
            let Ok((
                spine_mesh_entity, //
                mut spine_mesh,
                spine_3d_mesh,
            )) = mesh_query.get_mut(child)
            else {
                continue;
            };

            #[cfg(all(feature = "2d", feature = "3d"))]
            let Ok((
                spine_mesh_entity, //
                mut spine_mesh,
                spine_2d_mesh,
                spine_3d_mesh,
            )) = mesh_query.get_mut(child)
            else {
                continue;
            };

            let Ok(mut entity_commands) = commands.get_entity(spine_mesh_entity) else { continue };

            #[cfg(all(feature = "2d", not(feature = "3d")))]
            if spine_2d_mesh.is_none() {
                entity_commands.insert(Mesh2d(spine_mesh.handle.clone()));
            }

            #[cfg(all(feature = "3d", not(feature = "2d")))]
            if spine_3d_mesh.is_none() {
                entity_commands.insert(Mesh3d(spine_mesh.handle.clone()));
            }

            #[cfg(all(feature = "2d", feature = "3d"))]
            match mesh_type {
                SpineMeshType::Mesh2D => {
                    if spine_2d_mesh.is_none() {
                        entity_commands.insert(Mesh2d(spine_mesh.handle.clone()));
                    }

                    if spine_3d_mesh.is_some() {
                        entity_commands.remove::<Mesh3d>();
                    }
                }
                SpineMeshType::Mesh3D => {
                    if spine_3d_mesh.is_none() {
                        entity_commands.insert(Mesh3d(spine_mesh.handle.clone()));
                    }

                    if spine_2d_mesh.is_some() {
                        entity_commands.remove::<Mesh2d>();
                    }
                }
            }

            let Some(mesh) = meshes.get_mut(&spine_mesh.handle) else { continue };

            if let Some(attachment_renderer_object) = attachment_renderer_object {
                let texture = unsafe { &mut *(attachment_renderer_object as *mut SpineTexture) };
                let normals = vec![[0.; 3]; vertices.len()];

                mesh.insert_indices(Indices::U16(indices));
                mesh.insert_attribute(POSITION_ATTRIBUTE, vertices);
                mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
                mesh.insert_attribute(DARK_COLOR_ATTRIBUTE, dark_colors);

                spine_mesh.state = SpineMeshState::Renderable {
                    info: SpineMaterialInfo {
                        slot_index,
                        texture: asset_server.load(&texture.0),
                        blend_mode,
                        premultiplied_alpha,
                    },
                };
            } else {
                spine_mesh.state = SpineMeshState::Empty;

                remove_mesh(
                    entity_commands,
                    #[cfg(all(feature = "2d", feature = "3d"))]
                    mesh_type,
                );
            }
        }

        while let Some(child) = children.next() {
            let Ok((entity, mut spine_mesh, ..)) = mesh_query.get_mut(child) else { continue };

            if let SpineMeshState::Empty =
                mem::replace(&mut spine_mesh.state, SpineMeshState::Empty)
            {
                break;
            }

            if let Ok(entity_commands) = commands.get_entity(entity) {
                remove_mesh(
                    entity_commands,
                    #[cfg(all(feature = "2d", feature = "3d"))]
                    mesh_type,
                );
            }
        }
    }
}

const POSITION_ATTRIBUTE: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Position", 0, VertexFormat::Float32x2);

fn new_empty_mesh() -> Mesh {
    Mesh::new(PrimitiveTopology::TriangleList, default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0f32; 3]; 3])
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0f32; 3]; 3])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0f32; 2]; 3])
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0f32; 4]; 3])
        .with_inserted_attribute(DARK_COLOR_ATTRIBUTE, vec![[0f32; 4]; 3])
}

fn remove_mesh(
    mut entity_commands: EntityCommands,
    #[cfg(all(feature = "2d", feature = "3d"))] mesh_type: SpineMeshType,
) {
    #[cfg(all(feature = "2d", not(feature = "3d")))]
    entity_commands.remove::<Mesh2d>();

    #[cfg(all(feature = "3d", not(feature = "2d")))]
    entity_commands.remove::<Mesh3d>();

    #[cfg(all(feature = "2d", feature = "3d"))]
    match mesh_type {
        SpineMeshType::Mesh2D => entity_commands.remove::<Mesh2d>(),
        SpineMeshType::Mesh3D => entity_commands.remove::<Mesh3d>(),
    };
}

#[cfg(feature = "pma_fix")]
#[derive(Default)]
struct FixSpineTextures {
    handles: Vec<(Handle<Image>, SpineTextureConfig)>,
}

#[cfg(feature = "pma_fix")]
/// Adjusts Spine textures to render properly.
fn adjust_spine_textures(
    mut local: Local<FixSpineTextures>,
    mut spine_texture_create_events: MessageReader<SpineTextureCreateEvent>,
    mut images: ResMut<Assets<Image>>,
) {
    for spine_texture_create_event in spine_texture_create_events.read() {
        local.handles.push((
            spine_texture_create_event.handle.clone(),
            spine_texture_create_event.config,
        ));
    }

    let mut removed_handles = vec![];

    for (handle_index, (handle, handle_config)) in local.handles.iter().enumerate() {
        if let Some(image) = images.get_mut(handle) {
            fn convert_filter(filter: AtlasFilter) -> ImageFilterMode {
                match filter {
                    AtlasFilter::Nearest => ImageFilterMode::Nearest,
                    AtlasFilter::Linear => ImageFilterMode::Linear,
                    _ => {
                        #[cfg(feature = "debug")]
                        warn!("Unsupported Spine filter: {filter:?}",);
                        ImageFilterMode::Nearest
                    }
                }
            }

            fn convert_wrap(wrap: AtlasWrap) -> ImageAddressMode {
                match wrap {
                    AtlasWrap::ClampToEdge => ImageAddressMode::ClampToEdge,
                    AtlasWrap::MirroredRepeat => ImageAddressMode::MirrorRepeat,
                    AtlasWrap::Repeat => ImageAddressMode::Repeat,
                    _ => {
                        #[cfg(feature = "debug")]
                        warn!("Unsupported Spine wrap mode: {wrap:?}");
                        ImageAddressMode::ClampToEdge
                    }
                }
            }

            image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                min_filter: convert_filter(handle_config.min_filter),
                mag_filter: convert_filter(handle_config.mag_filter),
                address_mode_u: convert_wrap(handle_config.u_wrap),
                address_mode_v: convert_wrap(handle_config.v_wrap),
                ..default()
            });

            // The RGB components exported from Spine were premultiplied in nonlinear space, but need to be
            // multiplied in linear space to render properly in Bevy.
            if handle_config.premultiplied_alpha {
                if let Some(data) = &mut image.data {
                    for i in 0..(data.len() / 4) {
                        let mut rgba = Srgba::rgba_u8(
                            data[i * 4],
                            data[i * 4 + 1],
                            data[i * 4 + 2],
                            data[i * 4 + 3],
                        );

                        if rgba.alpha != 0. {
                            rgba = Srgba::new(
                                rgba.red / rgba.alpha,
                                rgba.green / rgba.alpha,
                                rgba.blue / rgba.alpha,
                                rgba.alpha,
                            );
                        } else {
                            rgba = Srgba::new(0., 0., 0., 0.);
                        }

                        let mut linear_rgba = LinearRgba::from(rgba);

                        linear_rgba.red *= linear_rgba.alpha;
                        linear_rgba.green *= linear_rgba.alpha;
                        linear_rgba.blue *= linear_rgba.alpha;

                        rgba = Srgba::from(linear_rgba);

                        data[i * 4] = (rgba.red * 255.) as u8;
                        data[i * 4 + 1] = (rgba.green * 255.) as u8;
                        data[i * 4 + 2] = (rgba.blue * 255.) as u8;
                        data[i * 4 + 3] = (rgba.alpha * 255.) as u8;
                    }
                }
            }

            removed_handles.push(handle_index);
        }
    }

    for removed_handle in removed_handles.into_iter().rev() {
        local.handles.remove(removed_handle);
    }
}

mod assets;
mod crossfades;
mod entity_sync;
mod handle;

pub mod materials;
pub mod textures;

#[cfg(test)]
mod test;

#[doc(hidden)]
pub mod prelude {
    pub use crate::{
        Crossfades, SkeletonController, SkeletonData, SkeletonDataHandle, Spine, SpineBone,
        SpineBundle, SpineEvent, SpineLoader, SpineMesh, SpineMeshState, SpinePlugin,
        SpineReadyEvent, SpineSet, SpineSettings, SpineSync, SpineSyncSet, SpineSyncSystem,
        SpineSystem,
    };
    pub use spine::{BoneHandle, SlotHandle};
}
