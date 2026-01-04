import { useRef, useState, useCallback } from 'react';
import './GlitchCard.css';

interface GlitchCardProps {
  icon: string;
  title: string;
  description?: string;
  onClick?: () => void;
}

/**
 * GlitchCard Component - Proper Card Structure with Glitch Text Effect
 *
 * Based on the reference implementation with:
 * - Outer card (wrapper) with background
 * - Outer glow revealed on grid hover
 * - Inner card (content) with 1px inset
 * - Inner glow on specific card hover
 * - Glitch text layer that fills background
 * - Mouse-following radial gradient mask
 */
export const GlitchCard: React.FC<GlitchCardProps> = ({
  icon,
  title,
  description,
  onClick,
}) => {
  const cardRef = useRef<HTMLDivElement>(null);
  const [glitchText, setGlitchText] = useState(generateRandomString(5000));
  const lastUpdateRef = useRef(0);

  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (!cardRef.current) return;

    const rect = cardRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    // Set CSS variables for glow effects
    const percentX = (x / rect.width) * 100;
    const percentY = (y / rect.height) * 100;
    (e.currentTarget as HTMLElement).style.setProperty('--mouse-x', `${percentX}%`);
    (e.currentTarget as HTMLElement).style.setProperty('--mouse-y', `${percentY}%`);

    // Throttle glitch text generation to max once every 100ms
    const now = Date.now();
    if (now - lastUpdateRef.current > 100) {
      setGlitchText(generateRandomString(5000));
      lastUpdateRef.current = now;
    }
  }, []);

  return (
    <div
      ref={cardRef}
      className="card"
      onMouseMove={handleMouseMove}
      onClick={onClick}
    >
      {/* Glitch text layer filling background */}
      <div className="glitch-text-layer">{glitchText}</div>

      {/* Inner card content */}
      <div className="card-content">
        <div className="card-icon">
          <i className={`fas ${icon}`}></i>
        </div>
        <h3 className="card-title">{title}</h3>
        {description && <p className="card-description">{description}</p>}
      </div>
    </div>
  );
};

function generateRandomString(length: number): string {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789@#$%^&*';
  let result = '';
  for (let i = 0; i < length; i++) {
    result += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return result;
}

export const GlitchCardGrid: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  return <div id="cards">{children}</div>;
};
