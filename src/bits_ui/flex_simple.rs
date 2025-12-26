use bevy::prelude::*;

#[derive(Component)]
#[require(Name)]
pub struct FlexSimple {
    width: Val,
    height: Val,
    flex_direction: FlexDirection,
    align_items: AlignItems,
    justify_content: JustifyContent,
}

impl FlexSimple {
    pub fn new() -> Self {
        Self {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
    }

    pub fn with_direction(mut self, direction: FlexDirection) -> Self {
        self.flex_direction = direction;
        self
    }

    pub fn with_size(mut self, width: Val, height: Val) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn bundle(self) -> impl Bundle {
        (
            Name::new("FlexSimple"),
            Node {
                width: self.width,
                height: self.height,
                flex_direction: self.flex_direction,
                align_items: self.align_items,
                justify_content: self.justify_content,
                ..default()
            },
            self,
        )
    }
}
