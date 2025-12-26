import { $ } from "bun";

interface WindowConfig {
  x: number;
  y: number;
}

export async function positionWindow(
  windowTitle: string,
  config: WindowConfig
): Promise<void> {
  const script = `
    tell application "System Events"
      set foundWindow to false
      repeat with proc in (every process whose visible is true)
        try
          if exists (window 1 of proc whose name is "${windowTitle}") then
            set position of window 1 of proc whose name is "${windowTitle}" to {${config.x}, ${config.y}}
            set foundWindow to true
            exit repeat
          end if
        end try
      end repeat
      if foundWindow is false then
        error "Window '${windowTitle}' not found"
      end if
    end tell
  `;

  try {
    const result = await $`osascript -e ${script}`.text();
    console.log(
      `✓ Positioned window "${windowTitle}" at (${config.x}, ${config.y})`
    );
  } catch (error: any) {
    console.error(`✗ Failed to position window "${windowTitle}"`);
    console.error(`  Error: ${error.stderr || error.message}`);
  }
}
