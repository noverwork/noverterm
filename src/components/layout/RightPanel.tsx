import { cn } from '@/lib/utils';
import { useSessionStore, usePortForwardStore } from '@/lib/stores';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { Badge } from '@/components/ui/badge';
import { Switch } from '@/components/ui/switch';
import { Button } from '@/components/ui/button';
import { Trash2, Plus, Copy } from 'lucide-react';
import { useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { toast } from 'sonner';

export function RightPanel() {
  const { activeSessionId, sessions, currentView } = useSessionStore();
  const { portForwards, togglePortForward, removePortForward, addPortForward } =
    usePortForwardStore();
  const [pfDialogOpen, setPfDialogOpen] = useState(false);

  const activeSession = sessions.find((s) => s.id === activeSessionId);
  const sessionPortForwards = activeSessionId?.toString
    ? portForwards.filter((pf) => pf.sessionId === activeSessionId)
    : [];

  if (!activeSession || currentView !== 'terminal') {
    return null;
  }

  const handleAddPortForward = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const name = formData.get('name') as string;
    const type = formData.get('type') as 'local' | 'remote' | 'dynamic';
    const localHost = formData.get('localHost') as string;
    const localPort = parseInt(formData.get('localPort') as string, 10);
    const remoteHost = formData.get('remoteHost') as string;
    const remotePort = formData.get('remotePort')
      ? parseInt(formData.get('remotePort') as string, 10)
      : undefined;

    if (!activeSessionId) return;

    await addPortForward({
      sessionId: activeSessionId,
      name,
      type,
      localHost,
      localPort,
      remoteHost,
      remotePort,
    });

    setPfDialogOpen(false);
    toast.success('Port forward added');
  };

  const handleCopy = (text: string) => {
    void navigator.clipboard.writeText(text);
    toast.success('Copied to clipboard');
  };

  return (
    <aside className="border-border bg-surface flex h-full w-80 flex-col border-l">
      <div className="border-border flex h-14 items-center border-b px-4">
        <h2 className="text-foreground font-semibold">Session Details</h2>
      </div>

      <ScrollArea className="flex-1">
        <div className="space-y-6 p-4">
          {/* Connection Info */}
          <div className="space-y-3">
            <h3 className="text-muted-foreground text-xs font-semibold uppercase">Connection</h3>
            <div className="space-y-2 text-sm">
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">Host</span>
                <span
                  className="text-foreground hover:text-primary cursor-pointer font-mono"
                  onClick={() => {
                    handleCopy(`${activeSession.username}@${activeSession.host}`);
                  }}
                >
                  {activeSession.host}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">Port</span>
                <span className="text-foreground font-mono">{activeSession.port}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">User</span>
                <span className="text-foreground font-mono">{activeSession.username}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">Auth</span>
                <Badge variant="secondary" className="text-xs">
                  {activeSession.authMethod}
                </Badge>
              </div>
            </div>
          </div>

          <Separator />

          {/* Port Forwards */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <h3 className="text-muted-foreground text-xs font-semibold uppercase">
                Port Forwards
              </h3>
              <Dialog open={pfDialogOpen} onOpenChange={setPfDialogOpen}>
                <DialogTrigger asChild>
                  <Button variant="ghost" size="icon" className="h-6 w-6">
                    <Plus className="h-3.5 w-3.5" />
                  </Button>
                </DialogTrigger>
                <DialogContent className="sm:max-w-md">
                  <form onSubmit={handleAddPortForward}>
                    <DialogHeader>
                      <DialogTitle>Add Port Forward</DialogTitle>
                      <DialogDescription>
                        Configure a new port forward rule for this session.
                      </DialogDescription>
                    </DialogHeader>
                    <div className="space-y-4 py-4">
                      <div className="space-y-2">
                        <Label htmlFor="pf-name">Name</Label>
                        <Input id="pf-name" name="name" placeholder="e.g., Database" required />
                      </div>
                      <div className="space-y-2">
                        <Label htmlFor="pf-type">Type</Label>
                        <Select name="type" defaultValue="local">
                          <SelectTrigger id="pf-type">
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="local">Local (-L)</SelectItem>
                            <SelectItem value="remote">Remote (-R)</SelectItem>
                            <SelectItem value="dynamic">Dynamic (-D)</SelectItem>
                          </SelectContent>
                        </Select>
                      </div>
                      <div className="grid grid-cols-2 gap-4">
                        <div className="space-y-2">
                          <Label htmlFor="pf-local-host">Local Host</Label>
                          <Input
                            id="pf-local-host"
                            name="localHost"
                            placeholder="localhost"
                            defaultValue="localhost"
                            required
                          />
                        </div>
                        <div className="space-y-2">
                          <Label htmlFor="pf-local-port">Local Port</Label>
                          <Input
                            id="pf-local-port"
                            name="localPort"
                            type="number"
                            placeholder="3306"
                            required
                          />
                        </div>
                      </div>
                      <div className="grid grid-cols-2 gap-4">
                        <div className="space-y-2">
                          <Label htmlFor="pf-remote-host">Remote Host</Label>
                          <Input id="pf-remote-host" name="remoteHost" placeholder="localhost" />
                        </div>
                        <div className="space-y-2">
                          <Label htmlFor="pf-remote-port">Remote Port</Label>
                          <Input
                            id="pf-remote-port"
                            name="remotePort"
                            type="number"
                            placeholder="3306"
                          />
                        </div>
                      </div>
                    </div>
                    <DialogFooter>
                      <Button type="submit">Add Forward</Button>
                    </DialogFooter>
                  </form>
                </DialogContent>
              </Dialog>
            </div>

            {sessionPortForwards.length === 0 ? (
              <p className="text-muted-foreground text-sm">No port forwards configured</p>
            ) : (
              <div className="space-y-2">
                {sessionPortForwards.map((pf) => (
                  <div
                    key={pf.id}
                    className="border-border bg-background flex items-center justify-between rounded-lg border p-3"
                  >
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        <span className="text-foreground text-sm font-medium">{pf.name}</span>
                        <Badge
                          variant="secondary"
                          className={cn(
                            'text-xs',
                            pf.type === 'local' && 'bg-blue-500/20 text-blue-400',
                            pf.type === 'remote' && 'bg-purple-500/20 text-purple-400',
                            pf.type === 'dynamic' && 'bg-green-500/20 text-green-400'
                          )}
                        >
                          {pf.type}
                        </Badge>
                      </div>
                      <div className="text-muted-foreground text-xs">
                        {pf.localHost}:{pf.localPort}
                        {pf.remoteHost && ` → ${pf.remoteHost}:${pf.remotePort}`}
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <Switch
                        checked={pf.active}
                        onCheckedChange={() => togglePortForward(pf.id)}
                        className="data-[state=checked]:bg-success"
                      />
                      <Button
                        variant="ghost"
                        size="icon"
                        className="text-muted-foreground hover:text-destructive h-6 w-6"
                        onClick={() => removePortForward(pf.id)}
                      >
                        <Trash2 className="h-3 w-3" />
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          <Separator />

          {/* Quick Actions */}
          <div className="space-y-3">
            <h3 className="text-muted-foreground text-xs font-semibold uppercase">Quick Actions</h3>
            <div className="space-y-2">
              <Button
                variant="outline"
                size="sm"
                className="border-border w-full justify-start"
                onClick={() => {
                  handleCopy(`ssh ${activeSession.username}@${activeSession.host}`);
                }}
              >
                <Copy className="mr-2 h-3.5 w-3.5" />
                Copy SSH Command
              </Button>
            </div>
          </div>
        </div>
      </ScrollArea>
    </aside>
  );
}
