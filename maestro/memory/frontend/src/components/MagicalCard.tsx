import { useRef, useState } from 'react';
import './MagicalCard.css';

/**
 * Effect 1: Magical Card Hover Effect
 *
 * - Black background page
 * - 3x2 grid of cards (6 total cards)
 * - Dark gray cards, slightly rounded, small gaps
 * - Faint white borders
 * - Dimmed white icon in center of each card
 * - Faint radial gradient glow follows mouse within active card
 * - Inner card (dark gray) nested 1px inside wrapper card (lighter)
 * - Creates visible "border" via inset
 * - Outer card has brighter glow that appears on ALL cards when ANY card is hovered
 * - Outer glow only visible through 1px inset border
 * - Makes it appear glow extends across neighboring cards
 */

interface MagicalCardProps {
  icon: string;
  title: string;
  description?: string;
  onClick?: () => void;
  className?: string;
  children?: React.ReactNode;
  isExpanded?: boolean;
}

export const MagicalCard: React.FC<MagicalCardProps> = ({
  icon,
  title,
  description,
  onClick,
  className = '',
  children,
  isExpanded = false,
}) => {
  const cardRef = useRef<HTMLDivElement>(null);
  const glowRef = useRef<HTMLDivElement>(null);
  const [mousePos, setMousePos] = useState({ x: 0, y: 0 });
  const [isHovered, setIsHovered] = useState(false);

  const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!cardRef.current) return;

    const rect = cardRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    setMousePos({ x, y });

    // Set CSS variables for grid-wide glow effect
    const percentX = (x / rect.width) * 100;
    const percentY = (y / rect.height) * 100;
    (e.currentTarget as HTMLElement).style.setProperty('--mouse-x', `${percentX}%`);
    (e.currentTarget as HTMLElement).style.setProperty('--mouse-y', `${percentY}%`);
  };

  return (
    <div
      ref={cardRef}
      className={`magical-card ${className} ${isHovered ? 'hovered' : ''} ${isExpanded ? 'expanded' : ''}`}
      onMouseMove={handleMouseMove}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      onClick={onClick}
    >
      {/* Outer wrapper card (lighter gray) */}
      <div className="card-outer">
        {/* 1px inset border that reveals the outer glow */}
        <div className="card-border-glow" />

        {/* Inner card (darker gray) */}
        <div className="card-inner">
          {/* Mouse-following radial gradient glow */}
          <div
            ref={glowRef}
            className="card-glow"
            style={{
              left: `${mousePos.x}px`,
              top: `${mousePos.y}px`,
            }}
          />

          {/* Card content */}
          <div className="card-content">
            <i className={`fas ${icon} card-icon`} />
            <h3 className="card-title">{title}</h3>
            {description && <p className="card-description">{description}</p>}
          </div>

          {/* Expanded content */}
          {children && (
            <div className="card-expanded">
              {children}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export const MagicalCardGrid: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  return (
    <div className="magical-card-grid">
      {children}
    </div>
  );
};
