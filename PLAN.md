# Need

## Bevy coroutines (this honestly might just be what i was missing)

## Layering

Need a good story for:

- bg1, bg2,...
- game
- fg1, fg2...
- menu

## Editing

- How to tweak UI and text layout in a way that isn't terrible...
  - Learn how to use hot reload?
  - Create my own editor?
  - Fixed size screen, exact positions?
  - Some other editor?
  - Learn how to use UI?

## Simplified animation system

Still want:

- Derive from aseprite files so tags are checked
- FPS, bullet time
- AnimMan<Enum> is good

ideal api:

anim!(EnumName, "path");

impl EnumName {
fn defaults(variant: Self) -> Thing {}
}

Don't need:

- Brightness, reflexitvity
- Layers should be handled separately
