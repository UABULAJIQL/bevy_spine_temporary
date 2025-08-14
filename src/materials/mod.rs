//! Materials for Spine meshes.
//!
//! To create a custom material for Spine, see [`SpineMaterial`].

#[cfg(feature = "2d")]
pub mod material_2d;
#[cfg(feature = "3d")]
pub mod material_3d;

use std::marker::PhantomData;

use bevy::prelude::*;

use bevy::asset::uuid_handle;
use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::mesh::MeshVertexAttribute;
use bevy::render::render_resource::VertexFormat;
use rusty_spine::BlendMode;

use crate::{SpineMesh, SpineMeshState, SpineSettings, SpineSystem};

/// Trait for automatically applying materials to [`SpineMesh`] entities. Used by the built-in
/// materials but can also be used to create custom materials.
///
/// Implement the trait and add it with [`SpineMaterialPlugin`].
pub trait SpineMaterial: Sized {
    type MeshMaterial: Component
        + Clone
        + Into<AssetId<Self::Material>>
        + From<Handle<Self::Material>>;
    /// The material type to apply to [`SpineMesh`]. Usually is `Self`.
    type Material: Asset + Clone;
    /// System parameters to query when updating this material.
    type Params<'w, 's>: SystemParam;

    /// Ran every frame for every material and every [`SpineMesh`].
    ///
    /// If this function returns [`Some`], then the material will be applied to the [`SpineMesh`],
    /// otherwise it will be removed. Default materials should be removed if a custom material is
    /// desired (see [`SpineSettings::default_materials`]).
    fn update(
        material: Option<Self::Material>,
        entity: Entity,
        renderable_data: SpineMaterialInfo,
        params: &StaticSystemParam<Self::Params<'_, '_>>,
    ) -> Option<Self::Material>;
}

/// Add support for a new [`SpineMaterial`].
pub struct SpineMaterialPlugin<T: SpineMaterial> {
    _marker: PhantomData<T>,
}

impl<T: SpineMaterial> Default for SpineMaterialPlugin<T> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<T: SpineMaterial + Send + Sync + 'static> Plugin for SpineMaterialPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_materials::<T>
                .in_set(SpineSystem::UpdateMaterials)
                .after(SpineSystem::UpdateMeshes),
        );
    }
}

/// Info necessary for a Spine material.
#[derive(Debug, Clone)]
pub struct SpineMaterialInfo {
    pub slot_index: Option<usize>,
    pub texture: Handle<Image>,
    pub blend_mode: BlendMode,
    pub premultiplied_alpha: bool,
}

#[allow(
    clippy::type_complexity,
    clippy::too_many_arguments
)]
fn update_materials<T: SpineMaterial>(
    mut commands: Commands,
    mut materials: ResMut<Assets<T::Material>>,
    mesh_query: Query<(Entity, &SpineMesh, Option<&T::MeshMaterial>)>,
    params: StaticSystemParam<T::Params<'_, '_>>,
) {
    for (mesh_entity, spine_mesh, material_handle) in mesh_query.iter() {
        let SpineMeshState::Renderable { info: data } = spine_mesh.state.clone() else { continue };

        if let Some((material, handle)) =
            material_handle.and_then(|handle| materials.get_mut(handle.clone()).zip(Some(handle)))
        {
            if let Some(new_material) = T::update(
                Some(material.clone()),
                spine_mesh.spine_entity,
                data,
                &params,
            ) {
                *material = new_material;
            } else {
                materials.remove(handle.clone());

                if let Ok(mut entity_commands) = commands.get_entity(mesh_entity) {
                    entity_commands.remove::<T::MeshMaterial>();
                }
            }
        } else if let Some(material) = T::update(None, spine_mesh.spine_entity, data, &params) {
            let handle = materials.add(material);

            if let Ok(mut entity_commands) = commands.get_entity(mesh_entity) {
                entity_commands.insert(Into::<T::MeshMaterial>::into(handle));
            }
        };
    }
}

/// A [`SystemParam`] to query [`SpineSettings`].
///
/// Mostly used for the built-in materials but may be useful for implementing other materials.
#[derive(SystemParam)]
pub struct SpineSettingsQuery<'w, 's> {
    pub spine_settings_query: Query<'w, 's, &'static SpineSettings>,
}

pub const DARK_COLOR_SHADER_POSITION: u64 = 10;
pub const DARK_COLOR_ATTRIBUTE: MeshVertexAttribute = MeshVertexAttribute::new(
    "Vertex_DarkColor",
    DARK_COLOR_SHADER_POSITION,
    VertexFormat::Float32x4,
);

pub const SHADER_HANDLE: Handle<Shader> = uuid_handle!("38b42512-1b99-43ed-ad09-6f36bb4ca3f9");
