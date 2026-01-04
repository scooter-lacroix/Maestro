import { useRef, useState, useEffect } from 'react';
import './GlitchText.css';

interface GlitchTextProps {
  text: string;
  className?: string;
  as?: 'h1' | 'h2' | 'h3' | 'span' | 'p';
}

export const GlitchText: React.FC<GlitchTextProps> = ({
  text,
  className = '',
  as: Component = 'h2',
}) => {
  const textRef = useRef<HTMLDivElement>(null);
  const gradientRef = useRef<HTMLDivElement>(null);
  const [isHovered, setIsHovered] = useState(false);
  const [randomText, setRandomText] = useState(text);
  const intervalRef = useRef<number>();

  const generateRandomText = (length: number): string => {
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
    let result = '';
    for (let i = 0; i < length; i++) {
      result += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return result;
  };

  const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!textRef.current) return;

    const rect = textRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    if (gradientRef.current) {
      gradientRef.current.style.left = `${x}px`;
      gradientRef.current.style.top = `${y}px`;
    }
  };

  const handleMouseEnter = () => {
    setIsHovered(true);

    intervalRef.current = window.setInterval(() => {
      setRandomText(generateRandomText(text.length));
    }, 50);
  };

  const handleMouseLeave = () => {
    setIsHovered(false);

    if (intervalRef.current) {
      clearInterval(intervalRef.current);
    }

    setTimeout(() => {
      setRandomText(text);
    }, 300);
  };

  useEffect(() => {
    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, []);

  return (
    <Component
      ref={textRef}
      className={`glitch-text-wrapper ${className} ${isHovered ? 'hovered' : ''}`}
      onMouseMove={handleMouseMove}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      <span className="glitch-text" style={{ opacity: isHovered ? 1 : 0 }}>
        {randomText}
      </span>
      <span className="original-text">{text}</span>
      <div
        ref={gradientRef}
        className="glitch-gradient"
        style={{ opacity: isHovered ? 0.4 : 0 }}
      />
    </Component>
  );
};
