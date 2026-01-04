import { useEffect, useRef } from 'react';
import './MouseTrailer.css';

/**
 * Effect 4: Fantastical Mouse Trailer Effect
 *
 * Core: soft continuous neon pink glow
 * Trail: falling stars using free Font Awesome fa-star icon
 * Stars alternate between neon pink and white
 * Glow follows mouse with short trail behind
 * Faster movement = more trail, but same disappear rate
 * Stars randomly appear around mouse position
 * Stars don't follow mouse - they spawn and immediately fall downward
 * Stars also rotate and fade out (~1 second animation)
 */
interface StarElement {
  element: HTMLDivElement;
  x: number;
  y: number;
  rotation: number;
  speedY: number;
  rotationSpeed: number;
  opacity: number;
  createdAt: number;
}

export const MouseTrailer: React.FC = () => {
  const glowRef = useRef<HTMLDivElement>(null);
  const starsRef = useRef<StarElement[]>([]);
  const mouseXRef = useRef(0);
  const mouseYRef = useRef(0);
  const animationFrameRef = useRef<number>();
  const lastSpawnTimeRef = useRef(0);
  const mouseSpeedRef = useRef(0);
  const lastMouseXRef = useRef(0);
  const lastMouseYRef = useRef(0);
  const isMouseInWindowRef = useRef(true);

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      mouseXRef.current = e.clientX;
      mouseYRef.current = e.clientY;
      isMouseInWindowRef.current = true;

      // Calculate mouse speed based on last position
      const dx = e.clientX - lastMouseXRef.current;
      const dy = e.clientY - lastMouseYRef.current;
      mouseSpeedRef.current = Math.sqrt(dx * dx + dy * dy);

      lastMouseXRef.current = e.clientX;
      lastMouseYRef.current = e.clientY;
    };

    const handleMouseLeave = () => {
      isMouseInWindowRef.current = false;
      // Clear all stars when mouse leaves window
      starsRef.current.forEach(star => {
        if (star.element.parentNode) {
          star.element.parentNode.removeChild(star.element);
        }
      });
      starsRef.current = [];

      // Hide glow immediately when mouse leaves
      if (glowRef.current) {
        glowRef.current.style.opacity = '0';
      }
    };

    const handleMouseEnter = () => {
      isMouseInWindowRef.current = true;
      // Show glow when mouse enters
      if (glowRef.current) {
        glowRef.current.style.opacity = '1';
      }
    };

    const animate = (timestamp: number) => {
      // DIRECT positioning - glow AT mouse position with minimal lag
      const targetX = mouseXRef.current;
      const targetY = mouseYRef.current;

      // Direct positioning with very tight follow - ALMOST at mouse position
      const lerpFactor = 0.8; // Very high for tight following
      const currentX = parseFloat(glowRef.current?.style.left || '0') || 0;
      const currentY = parseFloat(glowRef.current?.style.top || '0') || 0;

      const newX = currentX + (targetX - currentX) * lerpFactor;
      const newY = currentY + (targetY - currentY) * lerpFactor;

      if (glowRef.current) {
        glowRef.current.style.left = `${newX}px`;
        glowRef.current.style.top = `${newY}px`;

        // Only show glow if mouse is in window
        glowRef.current.style.opacity = isMouseInWindowRef.current ? '1' : '0';
      }

      // Spawn stars based on mouse speed (faster = more stars)
      const spawnThreshold = 100; // ms between spawns (increased for performance)
      const speedThreshold = 3; // minimum speed to spawn
      const spawnChance = Math.min(mouseSpeedRef.current / 15, 1);
      const maxStars = 50; // Maximum concurrent stars

      if (timestamp - lastSpawnTimeRef.current > spawnThreshold &&
          mouseSpeedRef.current > speedThreshold &&
          isMouseInWindowRef.current &&
          starsRef.current.length < maxStars) {
        if (Math.random() < spawnChance) {
          spawnStar(targetX, targetY);
          lastSpawnTimeRef.current = timestamp;
        }
      }

      // Update existing stars
      updateStars();

      // Decay mouse speed faster
      mouseSpeedRef.current *= 0.85;

      animationFrameRef.current = requestAnimationFrame(animate);
    };

    const spawnStar = (spawnX: number, spawnY: number) => {
      const star = document.createElement('div');
      star.className = 'trailer-star';
      star.innerHTML = '<i class="fas fa-star"></i>';

      // Random position around mouse (within 50px radius)
      const angle = Math.random() * Math.PI * 2;
      const radius = Math.random() * 50;
      const x = spawnX + Math.cos(angle) * radius;
      const y = spawnY + Math.sin(angle) * radius;

      // Random color (alternating pink and white)
      const isPink = Math.random() > 0.5;

      star.style.left = `${x}px`;
      star.style.top = `${y}px`;
      star.style.color = isPink ? '#ff10f0' : '#ffffff';

      document.body.appendChild(star);

      starsRef.current.push({
        element: star,
        x,
        y,
        rotation: Math.random() * 360,
        speedY: 2 + Math.random() * 3, // Fall speed
        rotationSpeed: (Math.random() - 0.5) * 15, // Faster rotation
        opacity: 1,
        createdAt: Date.now(),
      });
    };

    const updateStars = () => {
      const now = Date.now();
      const starsToRemove: number[] = [];

      starsRef.current.forEach((star, index) => {
        // Fall downward
        star.y += star.speedY;
        // Rotate
        star.rotation += star.rotationSpeed;
        // Fade out over 1 second
        const age = now - star.createdAt;
        star.opacity = Math.max(0, 1 - (age / 1000));

        // Update star position and rotation
        star.element.style.transform = `translate(-50%, -50%) rotate(${star.rotation}deg)`;
        star.element.style.left = `${star.x}px`;
        star.element.style.top = `${star.y}px`;
        star.element.style.opacity = star.opacity.toString();

        // Also hide all stars if mouse left window
        if (!isMouseInWindowRef.current) {
          star.element.style.opacity = '0';
        }

        // Mark for removal if faded out
        if (star.opacity <= 0) {
          starsToRemove.push(index);
        }
      });

      // Remove faded stars (in reverse order to maintain indices)
      for (let i = starsToRemove.length - 1; i >= 0; i--) {
        const index = starsToRemove[i];
        const star = starsRef.current[index];
        if (star.element.parentNode) {
          star.element.parentNode.removeChild(star.element);
        }
        starsRef.current.splice(index, 1);
      }
    };

    window.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseleave', handleMouseLeave);
    document.addEventListener('mouseenter', handleMouseEnter);
    animationFrameRef.current = requestAnimationFrame(animate);

    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseleave', handleMouseLeave);
      document.removeEventListener('mouseenter', handleMouseEnter);
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
      // Clean up stars
      starsRef.current.forEach(star => {
        if (star.element.parentNode) {
          star.element.parentNode.removeChild(star.element);
        }
      });
      starsRef.current = [];
    };
  }, []);

  return (
    <>
      <div ref={glowRef} className="mouse-glow" />
    </>
  );
};
