import { useEffect, useRef } from "react";
import { SkinViewer, WalkingAnimation } from "skinview3d";
import { getSkin } from "../lib/skin";

// Body angle (radians) — character turned toward the viewer's right (~34°).
// Shared everywhere so the sidebar, the big "current" render, and library tiles
// all match.
export const SKIN_ANGLE = 0.6;

export function SkinRender({
  uuid,
  src,
  model = "auto-detect",
  width,
  height,
  animated,
}: {
  /** Load the live skin for this player UUID (via the Mojang API). */
  uuid?: string;
  /** OR load a skin directly from a URL / bundled asset (Steve/Alex). */
  src?: string;
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

    const setup = (skinSrc: string) => {
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
    };

    if (src) {
      setup(src);
    } else if (uuid) {
      getSkin(uuid).then(setup).catch(() => {});
    }

    return () => {
      cancelled = true;
      viewer?.dispose();
    };
  }, [uuid, src, model, width, height, animated]);

  return (
    <canvas
      ref={canvasRef}
      className={`skin-canvas${animated ? " draggable" : ""}`}
    />
  );
}
