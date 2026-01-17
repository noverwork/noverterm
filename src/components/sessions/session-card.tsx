import { cn } from '@/lib/utils';
import type { SSHSession, SessionStatus } from '@/lib/types';
import { Card } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { useState } from 'react';
import { MoreVertical, Terminal, Key, Lock, Copy, Trash2, Edit } from 'lucide-react';
import { toast } from 'sonner';

const statusConfig: Record<SessionStatus, { label: string; className: string; icon: string }> = {
  Disconnected: {
    label: 'Disconnected',
    className: 'bg-muted text-muted-foreground',
    icon: '○',
  },
  Connecting: {
    label: 'Connecting...',
    className: 'bg-warning/20 text-warning animate-pulse',
    icon: '◐',
  },
  Connected: {
    label: 'Connected',
    className: 'bg-success/20 text-success',
    icon: '●',
  },
  Error: {
    label: 'Error',
    className: 'bg-destructive/20 text-destructive',
    icon: '✕',
  },
};

interface SessionCardProps {
  session: SSHSession;
  onConnect: (id: string) => void;
  onDisconnect: (id: string) => void;
  onEdit: (session: SSHSession) => void;
  onDelete: (id: string) => void;
}

export function SessionCard({
  session,
  onConnect,
  onDisconnect,
  onEdit,
  onDelete,
}: SessionCardProps) {
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const status = statusConfig[session.status];
  const canConnect = session.status === 'Disconnected' || session.status === 'Error';

  const handleCopy = () => {
    const sshCmd = `ssh ${session.username}@${session.host} -p ${session.port}`;
    void navigator.clipboard.writeText(sshCmd);
    toast.success('SSH command copied to clipboard');
  };

  const handleConnect = () => {
    if (session.authMethod === 'password') {
      // TODO: Show password dialog
      onConnect(session.id);
    } else {
      onConnect(session.id);
    }
  };

  return (
    <>
      <Card
        className={cn(
          'group border-border relative overflow-hidden transition-all duration-200',
          'hover:border-primary/50 hover:shadow-lg',
          session.status === 'Connected' && 'border-success/50 bg-success/5'
        )}
      >
        {/* Status Bar */}
        <div
          className={cn(
            'h-1 w-full',
            session.status === 'Connected' && 'bg-success',
            session.status === 'Connecting' && 'bg-warning animate-pulse',
            session.status === 'Error' && 'bg-destructive',
            session.status === 'Disconnected' && 'bg-muted'
          )}
        />

        <div className="p-4">
          {/* Header */}
          <div className="mb-3 flex items-start justify-between">
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <h3 className="text-foreground truncate text-base font-semibold">{session.name}</h3>
                <Badge variant="secondary" className={status.className}>
                  {status.label}
                </Badge>
              </div>
              {session.group && (
                <span className="text-muted-foreground text-xs">{session.group}</span>
              )}
            </div>

            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8 opacity-0 transition-opacity group-hover:opacity-100"
                >
                  <MoreVertical className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={handleConnect} disabled={!canConnect}>
                  <Terminal className="mr-2 h-4 w-4" />
                  Connect
                </DropdownMenuItem>
                <DropdownMenuItem
                  onClick={() => {
                    onEdit(session);
                  }}
                >
                  <Edit className="mr-2 h-4 w-4" />
                  Edit
                </DropdownMenuItem>
                <DropdownMenuItem onClick={handleCopy}>
                  <Copy className="mr-2 h-4 w-4" />
                  Copy SSH Command
                </DropdownMenuItem>
                <DropdownMenuItem
                  className="text-destructive"
                  onClick={() => {
                    setShowDeleteDialog(true);
                  }}
                >
                  <Trash2 className="mr-2 h-4 w-4" />
                  Delete
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>

          {/* Connection Info */}
          <div className="mb-4 space-y-1 text-sm">
            <div className="text-muted-foreground flex items-center gap-2">
              <span className="font-mono text-xs">
                {session.username}@{session.host}:{session.port}
              </span>
            </div>
            <div className="text-muted-foreground flex items-center gap-1 text-xs">
              {session.authMethod === 'key' && (
                <>
                  <Key className="h-3 w-3" />
                  <span>SSH Key</span>
                </>
              )}
              {session.authMethod === 'password' && (
                <>
                  <Lock className="h-3 w-3" />
                  <span>Password</span>
                </>
              )}
              {session.authMethod === 'agent' && (
                <>
                  <Key className="h-3 w-3" />
                  <span>SSH Agent</span>
                </>
              )}
            </div>
            {session.error && <p className="text-destructive text-xs">{session.error}</p>}
          </div>

          {/* Actions */}
          <div className="flex gap-2">
            {session.status === 'Connected' ? (
              <Button
                variant="outline"
                size="sm"
                onClick={() => {
                  onDisconnect(session.id);
                }}
                className="border-destructive/50 text-destructive hover:bg-destructive/10 flex-1"
              >
                Disconnect
              </Button>
            ) : session.status === 'Connecting' ? (
              <Button variant="outline" size="sm" disabled className="flex-1">
                Connecting...
              </Button>
            ) : (
              <Button
                variant="default"
                size="sm"
                onClick={handleConnect}
                className="bg-primary text-primary-foreground hover:bg-primary/90 flex-1"
              >
                <Terminal className="mr-1.5 h-3.5 w-3.5" />
                Connect
              </Button>
            )}
          </div>
        </div>
      </Card>

      {/* Delete Confirmation Dialog */}
      <AlertDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Session</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete the session "{session.name}"? This action cannot be
              undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                onDelete(session.id);
                setShowDeleteDialog(false);
              }}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
