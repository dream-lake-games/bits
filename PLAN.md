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

## How to structure this thing?

one option is to make it super generic, just have a position, list of pixel offsets, some floats to control the effect, then do it.

^i kinda like this because then i could reuse it for other things besides just letters (button outlines, hover states, etc)

then i could probably make text on top pretty simply by just having it some "on complete" call back that is called on the frame befoe we despawn stuff, and that should spawn the letter
