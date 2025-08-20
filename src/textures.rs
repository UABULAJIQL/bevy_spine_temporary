//! Events related to textures loaded by Spine.

use std::sync::{Arc, Mutex};

use bevy::prelude::*;

use rusty_spine::atlas::{AtlasFilter, AtlasWrap};

use crate::Atlas;

#[derive(Debug)]
pub struct SpineTexture(pub String);

#[derive(Debug)]
struct SpineTextureInternal {
    pub path: String,
    pub atlas_address: usize,
    pub config: SpineTextureConfig,
}

#[derive(Debug, Clone, Copy)]
pub struct SpineTextureConfig {
    pub min_filter: AtlasFilter,
    pub mag_filter: AtlasFilter,
    pub u_wrap: AtlasWrap,
    pub v_wrap: AtlasWrap,
    pub premultiplied_alpha: bool,
}

#[derive(Resource)]
pub(crate) struct SpineTextures {
    data: Arc<Mutex<SpineTexturesData>>,
}

/// An [`Event`] fired for each texture loaded by Spine.
///
/// Sent in [`SpineSystem::Load`](`crate::SpineSystem::Load`).
#[derive(Debug, Clone, BufferedEvent)]
pub struct SpineTextureCreateEvent {
    pub path: String,
    pub handle: Handle<Image>,
    pub atlas: AssetId<Atlas>,
    pub config: SpineTextureConfig,
}

/// An [`Event`] fired for each texture disposed, after [`SpineTextureCreateEvent`].
///
/// Sent in [`SpineSystem::Load`](`crate::SpineSystem::Load`).
#[derive(Debug, Clone, BufferedEvent)]
pub struct SpineTextureDisposeEvent {
    pub path: String,
    pub handle: Handle<Image>,
}

#[derive(Default)]
pub(crate) struct SpineTexturesData {
    handles: Vec<(String, Handle<Image>)>,
    remember: Vec<SpineTextureInternal>,
    forget: Vec<String>,
}

impl SpineTextures {
    pub(crate) fn init() -> Self {
        let data = Arc::new(Mutex::new(SpineTexturesData::default()));

        let data2 = data.clone();

        rusty_spine::extension::set_create_texture_cb(move |page, path| {
            data2.lock().unwrap().remember.push(SpineTextureInternal {
                path: path.to_owned(),
                atlas_address: page.atlas().c_ptr() as usize,
                config: SpineTextureConfig {
                    min_filter: page.min_filter(),
                    mag_filter: page.mag_filter(),
                    u_wrap: page.u_wrap(),
                    v_wrap: page.v_wrap(),
                    premultiplied_alpha: false,
                },
            });

            page.renderer_object().set(SpineTexture(path.to_owned()));
        });

        let data3 = data.clone();

        rusty_spine::extension::set_dispose_texture_cb(move |page| unsafe {
            data3.lock().unwrap().forget.push(
                page.renderer_object()
                    .get_unchecked::<SpineTexture>()
                    .0
                    .clone(),
            );

            page.renderer_object().dispose::<SpineTexture>();
        });

        Self { data }
    }

    pub fn update(
        &self,
        asset_server: &AssetServer,
        atlases: &Assets<Atlas>,
        create_events: &mut EventWriter<SpineTextureCreateEvent>,
        dispose_events: &mut EventWriter<SpineTextureDisposeEvent>,
    ) {
        let mut data = self.data.lock().unwrap();

        while let Some(SpineTextureInternal {
            path,
            atlas_address,
            mut config,
        }) = data.remember.pop()
        {
            let handle = asset_server.load(&path);

            // if none, the atlas was already deleted before getting here
            if let Some((
                atlas,
                &Atlas {
                    premultiplied_alpha,
                    ..
                },
            )) = atlases
                .iter()
                .find(|(_, atlas)| atlas.atlas.c_ptr() as usize == atlas_address)
            {
                config.premultiplied_alpha = premultiplied_alpha;

                data.handles.push((path.clone(), handle.clone()));
                create_events.write(SpineTextureCreateEvent {
                    path,
                    handle,
                    atlas,
                    config,
                });
            }
        }

        while let Some(path) = data.forget.pop() {
            if let Some(index) = data.handles.iter().position(|handle| handle.0 == path) {
                let (_, handle) = data.handles.swap_remove(index);

                dispose_events.write(SpineTextureDisposeEvent { path, handle });
            }
        }
    }
}
