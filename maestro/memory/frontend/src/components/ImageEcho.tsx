import './ImageEcho.css';

/**
 * Effect 3: Infinite Looping Image Echo Effect
 *
 * Particularly for the memories section
 * Single vertical image centered, object-fit and object-position center
 * Rounded corners
 * 8 identical copies stacked absolutely on top of each other
 * 9th image (top one) stays stationary
 * 8 images paired into 4 sets (left/right pairs)
 * Each pair animates: scale 1→0.9, translate 0→±25%, opacity 1→0
 * Animation: 3 seconds duration per image
 * Creates effect of moving backward and away while fading
 * 4 pairs staggered evenly so first ends when last completes
 * Infinite loop
 */

interface ImageEchoProps {
  src?: string;
  width?: number;
  height?: number;
  className?: string;
}

// Use the user's provided brain.png image
const DEFAULT_IMAGE = "/static/memory.png";

export const ImageEcho: React.FC<ImageEchoProps> = ({
  src = DEFAULT_IMAGE,
  width = 350,
  height = 525,
  className = '',
}) => {
  return (
    <div className={`echo-container ${className}`}>
      <div className="echo-wrapper" style={{ width, height }}>
        {/* Echo 1 Left */}
        <img
          className="echo-image echo-1-left"
          src={src}
          alt="Echo 1 Left"
        />
        {/* Echo 1 Right */}
        <img
          className="echo-image echo-1-right"
          src={src}
          alt="Echo 1 Right"
        />
        {/* Echo 2 Left */}
        <img
          className="echo-image echo-2-left"
          src={src}
          alt="Echo 2 Left"
        />
        {/* Echo 2 Right */}
        <img
          className="echo-image echo-2-right"
          src={src}
          alt="Echo 2 Right"
        />
        {/* Echo 3 Left */}
        <img
          className="echo-image echo-3-left"
          src={src}
          alt="Echo 3 Left"
        />
        {/* Echo 3 Right */}
        <img
          className="echo-image echo-3-right"
          src={src}
          alt="Echo 3 Right"
        />
        {/* Echo 4 Left */}
        <img
          className="echo-image echo-4-left"
          src={src}
          alt="Echo 4 Left"
        />
        {/* Echo 4 Right */}
        <img
          className="echo-image echo-4-right"
          src={src}
          alt="Echo 4 Right"
        />
        {/* Main stationary image */}
        <img
          className="echo-image main-image"
          src={src}
          alt="Main Image"
        />
      </div>
    </div>
  );
};
