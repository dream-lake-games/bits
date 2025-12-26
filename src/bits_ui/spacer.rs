use bevy::prelude::*;

#[derive(Component)]
#[require(Name)]
pub struct Spacer {
    width: Val,
    height: Val,
}

impl Spacer {
    pub fn height(height: Val) -> Self {
        Self {
            width: Val::Auto,
            height,
        }
    }

    pub fn width(width: Val) -> Self {
        Self {
            width,
            height: Val::Auto,
        }
    }

    pub fn size(width: Val, height: Val) -> Self {
        Self { width, height }
    }

    pub fn bundle(self) -> impl Bundle {
        (
            Node {
                width: self.width,
                height: self.height,
                ..default()
            },
            self,
        )
    }
}
