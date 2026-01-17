import { usePortForwardStore, useSessionStore } from '@/lib/stores';
import { PortForwardDialog } from './port-forward-dialog';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Card } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Switch } from '@/components/ui/switch';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { EmptyState } from '@/components/ui/empty-state';
import { Trash2, ArrowRightLeft, ArrowRight } from 'lucide-react';
import { cn } from '@/lib/utils';

const typeIcons = {
  local: ArrowRightLeft,
  remote: ArrowRightLeft,
  dynamic: ArrowRight,
};

const typeConfig = {
  local: { label: 'Local', description: '-L', color: 'bg-blue-500/20 text-blue-400' },
  remote: { label: 'Remote', description: '-R', color: 'bg-purple-500/20 text-purple-400' },
  dynamic: { label: 'Dynamic', description: '-D', color: 'bg-green-500/20 text-green-400' },
};

export function PortForwardList() {
  const { portForwards, togglePortForward, removePortForward } = usePortForwardStore();
  const { sessions } = useSessionStore();

  const getSessionName = (sessionId: string) => {
    const session = sessions.find((s) => s.id === sessionId);
    return session?.name ?? sessionId;
  };

  const handleDelete = (id: string) => {
    void removePortForward(id);
  };

  const handleToggle = (id: string) => {
    void togglePortForward(id);
  };

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="border-border flex items-center justify-between border-b px-6 py-4">
        <div>
          <h2 className="text-foreground text-lg font-semibold">Port Forwards</h2>
          <p className="text-muted-foreground text-sm">
            Manage SSH tunneling and port forwarding rules
          </p>
        </div>
        <PortForwardDialog />
      </div>

      {/* Port Forwards Table */}
      <ScrollArea className="flex-1">
        <div className="p-6">
          {portForwards.length === 0 ? (
            <EmptyState
              icon={
                <div className="bg-primary/20 flex h-16 w-16 items-center justify-center rounded-full">
                  <ArrowRightLeft className="text-primary h-8 w-8" />
                </div>
              }
              title="No port forwards configured"
              description="Add a port forward rule to tunnel traffic through SSH"
              action={<PortForwardDialog />}
            />
          ) : (
            <Card className="border-border">
              <Table>
                <TableHeader>
                  <TableRow className="border-border hover:bg-transparent">
                    <TableHead className="text-muted-foreground">Name</TableHead>
                    <TableHead className="text-muted-foreground">Type</TableHead>
                    <TableHead className="text-muted-foreground">Mapping</TableHead>
                    <TableHead className="text-muted-foreground">Session</TableHead>
                    <TableHead className="text-muted-foreground">Status</TableHead>
                    <TableHead className="text-muted-foreground text-right">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {portForwards.map((pf) => {
                    const Icon = typeIcons[pf.type];
                    const config = typeConfig[pf.type];

                    return (
                      <TableRow key={pf.id} className="border-border">
                        <TableCell className="text-foreground font-medium">{pf.name}</TableCell>
                        <TableCell>
                          <Badge variant="secondary" className={config.color}>
                            <Icon className="mr-1 h-3 w-3" />
                            {config.label}
                          </Badge>
                        </TableCell>
                        <TableCell>
                          <code className="text-xs">
                            {pf.localHost}:{pf.localPort}
                            {pf.remoteHost && (
                              <>
                                {' '}
                                {pf.type === 'remote' ? '←' : '→'} {pf.remoteHost}:{pf.remotePort}
                              </>
                            )}
                          </code>
                        </TableCell>
                        <TableCell className="text-muted-foreground">
                          {getSessionName(pf.sessionId)}
                        </TableCell>
                        <TableCell>
                          <div className="flex items-center gap-2">
                            <Switch
                              checked={pf.active}
                              onCheckedChange={() => {
                                handleToggle(pf.id);
                              }}
                              className="data-[state=checked]:bg-success"
                            />
                            <span
                              className={cn(
                                'text-xs',
                                pf.active ? 'text-success' : 'text-muted-foreground'
                              )}
                            >
                              {pf.active ? 'Active' : 'Inactive'}
                            </span>
                          </div>
                        </TableCell>
                        <TableCell className="text-right">
                          <Button
                            variant="ghost"
                            size="icon"
                            className="text-muted-foreground hover:text-destructive h-8 w-8"
                            onClick={() => {
                              handleDelete(pf.id);
                            }}
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </Card>
          )}
        </div>
      </ScrollArea>
    </div>
  );
}
