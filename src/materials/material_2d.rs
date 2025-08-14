use bevy::prelude::*;

use bevy::asset::Asset;
use bevy::ecs::system::StaticSystemParam;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, RenderPipelineDescriptor,
    SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use bevy::sprite::{AlphaMode2d, Material2d, Material2dKey};
use rusty_spine::BlendMode;

macro_rules! material {
    ($(#[$($attrss:tt)*])* $name:ident, $blend_mode:expr, $premultiplied_alpha:expr, $blend_state:expr) => {
        $(#[$($attrss)*])*
        #[derive(Asset, Default, AsBindGroup, TypePath, Clone)]
        pub struct $name {
            #[texture(0)]
            #[sampler(1)]
            pub image: Handle<Image>,
        }

        impl $name {
            pub fn new(image: Handle<Image>) -> Self {
                Self { image }
            }
        }

        impl Material2d for $name {
            fn vertex_shader() -> ShaderRef {
                super::SHADER_HANDLE.into()
            }

            fn fragment_shader() -> ShaderRef {
                super::SHADER_HANDLE.into()
            }

            fn alpha_mode(&self) -> AlphaMode2d {
                AlphaMode2d::Blend
            }

            fn specialize(
                descriptor: &mut RenderPipelineDescriptor,
                layout: &MeshVertexBufferLayoutRef,
                _key: Material2dKey<Self>,
            ) -> Result<(), SpecializedMeshPipelineError> {
                let vertex_attributes = vec![
                    Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
                    Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
                    Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
                    Mesh::ATTRIBUTE_COLOR.at_shader_location(4),
                    super::DARK_COLOR_ATTRIBUTE.at_shader_location(super::DARK_COLOR_SHADER_POSITION as u32),
                ];

                let vertex_buffer_layout = layout.0.get_layout(&vertex_attributes)?;
                descriptor.vertex.buffers = vec![vertex_buffer_layout];

                if let Some(fragment) = &mut descriptor.fragment {
                    if let Some(target_state) = &mut fragment.targets[0] {
                        target_state.blend = Some($blend_state);
                    }
                }

                descriptor.primitive.cull_mode = None;
                Ok(())
            }
        }

        impl super::SpineMaterial for $name {
            type MeshMaterial = MeshMaterial2d<Self>;
            type Material = Self;
            type Params<'w, 's> = super::SpineSettingsQuery<'w, 's>;

            fn update(
                material: Option<Self>,
                entity: Entity,
                renderable_data: super::SpineMaterialInfo,
                params: &StaticSystemParam<Self::Params<'_, '_>>,
            ) -> Option<Self> {
                let spine_settings = params.spine_settings_query.get(entity).copied().unwrap_or_default();

                if spine_settings.default_materials
                    && renderable_data.blend_mode == $blend_mode
                    && renderable_data.premultiplied_alpha == $premultiplied_alpha
                {
                    let mut material = material.unwrap_or_default();
                    material.image = renderable_data.texture;

                    Some(material)
                } else {
                    None
                }
            }
        }
    };
}

#[cfg(feature = "normal_material")]
material!(
    /// Normal blend mode material, non-premultiplied-alpha
    SpineNormalMaterial,
    BlendMode::Normal,
    false,
    BlendState {
        color: BlendComponent {
            src_factor: BlendFactor::SrcAlpha,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
        alpha: BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
    }
);

#[cfg(feature = "additive_material")]
material!(
    /// Additive blend mode material, non-premultiplied-alpha
    SpineAdditiveMaterial,
    BlendMode::Additive,
    false,
    BlendState {
        color: BlendComponent {
            src_factor: BlendFactor::SrcAlpha,
            dst_factor: BlendFactor::One,
            operation: BlendOperation::Add,
        },
        alpha: BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::One,
            operation: BlendOperation::Add,
        },
    }
);

#[cfg(feature = "multiply_material")]
material!(
    /// Multiply blend mode material, non-premultiplied-alpha
    SpineMultiplyMaterial,
    BlendMode::Multiply,
    false,
    BlendState {
        color: BlendComponent {
            src_factor: BlendFactor::Dst,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
        alpha: BlendComponent {
            src_factor: BlendFactor::OneMinusSrcAlpha,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
    }
);

#[cfg(feature = "screen_material")]
material!(
    /// Screen blend mode material, non-premultiplied-alpha
    SpineScreenMaterial,
    BlendMode::Screen,
    false,
    BlendState {
        color: BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
        alpha: BlendComponent {
            src_factor: BlendFactor::OneMinusSrc,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
    }
);

#[cfg(feature = "normal_pma_material")]
material!(
    /// Normal blend mode material, premultiplied-alpha
    SpineNormalPmaMaterial,
    BlendMode::Normal,
    true,
    BlendState {
        color: BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
        alpha: BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
    }
);

#[cfg(feature = "additive_pma_material")]
material!(
    /// Additive blend mode material, premultiplied-alpha
    SpineAdditivePmaMaterial,
    BlendMode::Additive,
    true,
    BlendState {
        color: BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::One,
            operation: BlendOperation::Add,
        },
        alpha: BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::One,
            operation: BlendOperation::Add,
        },
    }
);

#[cfg(feature = "multiply_pma_material")]
material!(
    /// Multiply blend mode material, premultiplied-alpha
    SpineMultiplyPmaMaterial,
    BlendMode::Multiply,
    true,
    BlendState {
        color: BlendComponent {
            src_factor: BlendFactor::Dst,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
        alpha: BlendComponent {
            src_factor: BlendFactor::OneMinusSrcAlpha,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
    }
);

#[cfg(feature = "screen_pma_material")]
material!(
    /// Screen blend mode material, premultiplied-alpha
    SpineScreenPmaMaterial,
    BlendMode::Screen,
    true,
    BlendState {
        color: BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
        alpha: BlendComponent {
            src_factor: BlendFactor::OneMinusSrc,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
    }
);
