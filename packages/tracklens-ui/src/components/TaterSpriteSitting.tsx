/**
 * TrackLens UI - Tater Sprite (Sitting)
 *
 * Animated sprite component for the TrackLens mascot sitting.
 * Note: Requires sprite image assets.
 *
 * REBRANDED: Plannotator → TrackLens
 */

import React from 'react';

const NATIVE_SIZE = 96;
const DISPLAY_SIZE = 64;
const FRAMES = 12;
const SCALE = DISPLAY_SIZE / NATIVE_SIZE;
const TOTAL_WIDTH = NATIVE_SIZE * FRAMES * SCALE;

export const TaterSpriteSitting: React.FC = () => {
  return (
    <div
      className="hidden md:block absolute pointer-events-none z-10"
      style={{
        top: -40,
        right: -4,
        width: DISPLAY_SIZE,
        height: DISPLAY_SIZE,
        backgroundPosition: 'left center',
        animation: 'tater-sit 3s steps(12) infinite',
        imageRendering: 'pixelated',
      }}
    >
      <style>{`
        @keyframes tater-sit {
          to {
            background-position: -${TOTAL_WIDTH}px 0;
          }
        }
      `}</style>
    </div>
  );
};
