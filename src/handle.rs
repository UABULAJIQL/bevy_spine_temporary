use bevy::prelude::*;

use crate::SkeletonData;

#[derive(Default, Component)]
pub struct SkeletonDataHandle(pub Handle<SkeletonData>);

impl From<Handle<SkeletonData>> for SkeletonDataHandle {
    fn from(handle: Handle<SkeletonData>) -> Self {
        Self(handle)
    }
}
