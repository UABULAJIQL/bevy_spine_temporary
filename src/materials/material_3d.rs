use bevy::prelude::*;

use spine::BlendMode;

#[cfg(feature = "2d")]
use crate::SpineMeshType;

#[derive(Component)]
pub struct Spine3DMaterial;

impl super::SpineMaterial for Spine3DMaterial {
    type MeshMaterial = MeshMaterial3d<StandardMaterial>;
    type Material = StandardMaterial;
    type Params<'w, 's> = super::SpineSettingsQuery<'w, 's>;

    fn update(
        material: Option<Self::Material>,
        entity: Entity,
        renderable_data: super::SpineMaterialInfo,
        params: &super::StaticSystemParam<Self::Params<'_, '_>>,
    ) -> Option<Self::Material> {
        match params.spine_settings_query.get(entity) {
            Ok(spine_settings)
                if {
                    #[cfg(not(feature = "2d"))]
                    {
                        spine_settings.default_materials
                    }
                    #[cfg(feature = "2d")]
                    {
                        spine_settings.default_materials
                            && spine_settings.mesh_type == SpineMeshType::Mesh3D
                    }
                } =>
            {
                let mut material = material.unwrap_or(Self::Material {
                    unlit: true,
                    alpha_mode: match renderable_data.blend_mode {
                        BlendMode::Normal => AlphaMode::Blend,
                        BlendMode::Multiply => AlphaMode::Multiply,
                        _ => AlphaMode::Add,
                    },
                    ..default()
                });

                material.base_color_texture = Some(renderable_data.texture);
                Some(material)
            }
            _ => None,
        }
    }
}
