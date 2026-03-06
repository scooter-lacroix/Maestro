/**
 * TrackLens UI - Tater Sprite (Pullup)
 *
 * Animated sprite component for the TrackLens mascot.
 * Note: Requires sprite image assets.
 *
 * REBRANDED: Plannotator → TrackLens
 */

import React from 'react';

const NATIVE_SIZE = 96;
const DISPLAY_SIZE = 56;
const FRAMES = 24;
const SCALE = DISPLAY_SIZE / NATIVE_SIZE;
const TOTAL_WIDTH = NATIVE_SIZE * FRAMES * SCALE;

export const TaterSpritePullup: React.FC = () => {
  return (
    <div
      className="absolute pointer-events-none hidden md:block -z-10"
      style={{
        bottom: -49,
        left: 12,
        width: DISPLAY_SIZE,
        height: DISPLAY_SIZE,
        animation: 'tater-pullup 3.5s steps(24) infinite',
        imageRendering: 'pixelated',
      }}
    >
      <style>{`
        @keyframes tater-pullup {
          to { background-position: -${TOTAL_WIDTH}px 0; }
        }
      `}</style>
    </div>
  );
};
