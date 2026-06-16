import { useEffect, useRef } from "react";
import { SkinViewer, WalkingAnimation } from "skinview3d";
import { getPlayerTextures } from "../lib/skin";

// Body angle (radians) — character turned toward the viewer's right (~34°).
// Shared everywhere so the sidebar, the big "current" render, and library tiles
// all match.
export const SKIN_ANGLE = 0.6;

export function SkinRender({
  uuid,
  src,
  cape,
  model = "auto-detect",
  width,
  height,
  animated,
}: {
  /** Load the live skin for this player UUID (via the Mojang API). */
  uuid?: string;
  /** OR load a skin directly from a URL / bundled asset (Steve/Alex). */
  src?: string;
  /** Optional cape texture (full PNG data URL) to drape on the model. */
  cape?: string | null;
  model?: "auto-detect" | "default" | "slim";
  width: number;
  height: number;
  animated: boolean;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    let viewer: SkinViewer | null = null;
    let cancelled = false;

    const setup = (skinSrc: string, capeSrc?: string | null) => {
      if (cancelled || !canvasRef.current) return;
      viewer = new SkinViewer({ canvas: canvasRef.current, width, height });
      viewer.zoom = 0.85;

      const anim = new WalkingAnimation();
      anim.speed = 0.55;
      anim.paused = !animated; // paused (frozen) unless animated
      viewer.animation = anim;

      viewer.controls.enableZoom = false;
      viewer.controls.enablePan = false;
      viewer.controls.enableRotate = animated;

      // Apply the angle AFTER the skin loads — loadSkin rebuilds the model and
      // would otherwise reset the rotation we set.
      viewer.loadSkin(skinSrc, { model }).then(() => {
        if (cancelled || !viewer) return;
        viewer.playerObject.rotation.y = SKIN_ANGLE;
      });
      if (capeSrc) {
        viewer.loadCape(capeSrc).catch(() => {});
      }
    };

    if (src) {
      setup(src, cape);
    } else if (uuid) {
      getPlayerTextures(uuid)
        .then((t) => setup(t.skin, t.cape))
        .catch(() => {});
    }

    return () => {
      cancelled = true;
      viewer?.dispose();
    };
  }, [uuid, src, cape, model, width, height, animated]);

  return (
    <canvas
      ref={canvasRef}
      className={`skin-canvas${animated ? " draggable" : ""}`}
      // Reserve the exact size up front so a remount (e.g. switching skins)
      // can't briefly collapse the canvas and shift the layout.
      style={{ width, height }}
    />
  );
}
