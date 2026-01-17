import { cn } from '@/lib/utils';
import { useSessionStore } from '@/lib/stores';
import { X, Split, Square } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import type { SessionStatus } from '@/lib/types';

const statusConfig: Record<SessionStatus, { label: string; className: string }> = {
  Disconnected: { label: 'Disconnected', className: 'bg-muted text-muted-foreground' },
  Connecting: { label: 'Connecting...', className: 'bg-warning text-warning-foreground' },
  Connected: { label: 'Connected', className: 'bg-success text-success-foreground' },
  Error: { label: 'Error', className: 'bg-destructive text-destructive-foreground' },
};

interface TopBarProps {
  onSplit?: () => void;
}

export function TopBar({ onSplit }: TopBarProps) {
  const { currentView, activeSessionId, sessions, disconnectSession, setActiveSession } =
    useSessionStore();

  const activeSession = sessions.find((s) => s.id === activeSessionId);

  const getTitle = () => {
    if (currentView === 'terminal' && activeSession) {
      return activeSession.name;
    }
    switch (currentView) {
      case 'dashboard':
        return 'Sessions';
      case 'keys':
        return 'SSH Keys';
      case 'portForwards':
        return 'Port Forwards';
      default:
        return 'Noverterm';
    }
  };

  const handleDisconnect = async () => {
    if (activeSessionId) {
      await disconnectSession(activeSessionId);
      setActiveSession(null);
    }
  };

  return (
    <header className="border-border bg-surface flex h-14 items-center justify-between border-b px-4">
      {/* Left - Title */}
      <div className="flex items-center gap-3">
        <h1 className="text-foreground text-lg font-semibold">{getTitle()}</h1>
        {activeSession && (
          <Badge variant="secondary" className={statusConfig[activeSession.status].className}>
            {statusConfig[activeSession.status].label}
          </Badge>
        )}
      </div>

      {/* Right - Actions */}
      <div className="flex items-center gap-2">
        {currentView === 'terminal' && activeSession && (
          <>
            <Button variant="outline" size="sm" onClick={onSplit} className="border-border h-8">
              <Split className="mr-1.5 h-3.5 w-3.5" />
              Split
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={handleDisconnect}
              className="border-border hover:border-destructive hover:text-destructive h-8"
            >
              <Square className="mr-1.5 h-3.5 w-3.5" />
              Disconnect
            </Button>
          </>
        )}
        {(currentView === 'dashboard' || currentView === 'terminal') && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              setActiveSession(null);
            }}
            className={cn(
              'border-border h-8',
              activeSessionId && 'hover:border-destructive hover:text-destructive'
            )}
          >
            <X className="h-4 w-4" />
          </Button>
        )}
      </div>
    </header>
  );
}
