import { useEffect, useRef, useCallback } from 'react';
import { cn } from '@/lib/utils';

interface TerminalCanvasProps {
  sessionId: string;
  onData?: (data: string) => void;
  className?: string;
}

export function TerminalCanvas({ sessionId, onData, className }: TerminalCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Terminal state
  const rows = 24;
  const cols = 80;
  const charWidth = 9;
  const charHeight = 18;

  // Render terminal
  const renderTerminal = useCallback(
    (ctx: CanvasRenderingContext2D, width: number, height: number) => {
      // Background
      ctx.fillStyle = '#0F172A'; // slate-900
      ctx.fillRect(0, 0, width, height);

      // Font settings
      ctx.font = `${String(charHeight)}px 'JetBrains Mono', monospace`;
      ctx.textBaseline = 'top';

      // Draw welcome message
      ctx.fillStyle = '#F1F5F9'; // slate-100
      const lines = [
        'Noverterm v0.1.0',
        '',
        'Welcome to Noverterm - SSH Terminal Manager',
        `Session: ${sessionId}`,
        '',
        'Press any key to start typing...',
      ];

      lines.forEach((line, i) => {
        ctx.fillText(line, 10, i * charHeight + 10);
      });

      // Cursor
      const cursorY = 6 * charHeight + 10;
      ctx.fillStyle = '#3B82F6'; // blue-500
      ctx.fillRect(10, cursorY, charWidth, 2);
    },
    [charHeight, sessionId]
  );

  // Initialize canvas
  useEffect(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Set canvas size
    const width = cols * charWidth;
    const height = rows * charHeight;
    canvas.width = width;
    canvas.height = height;

    // Initial render
    renderTerminal(ctx, width, height);

    // Handle keyboard input
    const handleKeyDown = (e: KeyboardEvent) => {
      // Prevent default for terminal keys
      if (e.key === 'Enter' || e.key === 'Tab' || e.key === 'Backspace') {
        e.preventDefault();
      }

      // Send to Rust backend
      // const keyData = new TextEncoder().encode(e.key);
      // TODO: invoke("send_input", { sessionId, data: Array.from(keyData) })

      onData?.(e.key);
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [sessionId, onData, renderTerminal, charHeight, cols, rows]);

  return (
    <div ref={containerRef} className={cn('relative', className)}>
      <canvas
        ref={canvasRef}
        className="rounded-lg bg-slate-900"
        style={{ imageRendering: 'pixelated' }}
      />
    </div>
  );
}

// Terminal data types for Canvas rendering
export interface TerminalCell {
  ch: string;
  fg: number; // RGB hex
  bg: number;
  bold: boolean;
  italic: boolean;
  underline: boolean;
}

export interface TerminalGrid {
  rows: number;
  cols: number;
  cells: TerminalCell[][];
  cursorRow: number;
  cursorCol: number;
}
