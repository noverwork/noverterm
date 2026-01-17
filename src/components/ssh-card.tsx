import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Terminal, Wifi, WifiOff, Loader2, Trash2 } from 'lucide-react';
import { cn } from '@/lib/utils';

export interface SSHSession {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  status: 'Disconnected' | 'Connecting' | 'Connected' | 'Error';
}

interface SSHCardProps {
  session: SSHSession;
  onConnect: (id: string) => void;
  onDisconnect: (id: string) => void;
  onDelete: (id: string) => void;
}

const statusConfig = {
  Disconnected: {
    icon: WifiOff,
    color: 'bg-slate-500',
    textColor: 'text-slate-400',
    label: 'Disconnected',
  },
  Connecting: {
    icon: Loader2,
    color: 'bg-yellow-500',
    textColor: 'text-yellow-400',
    label: 'Connecting...',
  },
  Connected: {
    icon: Wifi,
    color: 'bg-green-500',
    textColor: 'text-green-400',
    label: 'Connected',
  },
  Error: {
    icon: WifiOff,
    color: 'bg-red-500',
    textColor: 'text-red-400',
    label: 'Error',
  },
};

export function SSHCard({ session, onConnect, onDisconnect, onDelete }: SSHCardProps) {
  const config = statusConfig[session.status];
  const StatusIcon = config.icon;

  return (
    <Card className="group border-slate-700 bg-slate-800/50 transition-colors hover:border-blue-500/50">
      <div className="p-4">
        <div className="mb-3 flex items-start justify-between">
          <div className="flex items-center gap-3">
            <div
              className={cn(
                'rounded-lg p-2',
                session.status === 'Connected' ? 'bg-green-500/10' : 'bg-slate-700/50'
              )}
            >
              <Terminal
                className={cn(
                  'h-4 w-4',
                  session.status === 'Connected' ? 'text-green-400' : 'text-slate-400'
                )}
              />
            </div>
            <div>
              <h3 className="font-medium text-slate-100">{session.name}</h3>
              <p className="font-mono text-xs text-slate-500">
                {session.username}@{session.host}:{session.port}
              </p>
            </div>
          </div>
          <Button
            variant="ghost"
            size="icon"
            className="text-slate-400 opacity-0 transition-opacity group-hover:opacity-100 hover:text-red-400"
            onClick={() => {
              onDelete(session.id);
            }}
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>

        <div className="flex items-center justify-between">
          <Badge
            variant="outline"
            className={cn(
              'gap-1.5 border-0',
              session.status === 'Connected'
                ? 'bg-green-500/10 text-green-400'
                : session.status === 'Connecting'
                  ? 'bg-yellow-500/10 text-yellow-400'
                  : 'bg-slate-500/10 text-slate-400'
            )}
          >
            <StatusIcon
              className={cn('h-3 w-3', session.status === 'Connecting' && 'animate-spin')}
            />
            {config.label}
          </Badge>

          {session.status === 'Connected' ? (
            <Button
              variant="outline"
              size="sm"
              className="h-7 border-slate-700 text-xs text-slate-300 hover:bg-slate-700"
              onClick={() => {
                onDisconnect(session.id);
              }}
            >
              Disconnect
            </Button>
          ) : (
            <Button
              size="sm"
              className="h-7 bg-blue-600 text-xs hover:bg-blue-700"
              onClick={() => {
                onConnect(session.id);
              }}
              disabled={session.status === 'Connecting'}
            >
              {session.status === 'Connecting' ? (
                <>
                  <Loader2 className="mr-1 h-3 w-3 animate-spin" />
                  Connecting...
                </>
              ) : (
                'Connect'
              )}
            </Button>
          )}
        </div>
      </div>
    </Card>
  );
}
