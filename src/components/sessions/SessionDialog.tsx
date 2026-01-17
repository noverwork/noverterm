import { useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Label } from '@/components/ui/label';
import { useSessionStore, useKeyStore } from '@/lib/stores';
import { type AuthMethod } from '@/lib/types';
import { Lock, Key } from 'lucide-react';

interface SessionDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  editingSession?: {
    id: string;
    name: string;
    group?: string;
    host: string;
    port: number;
    username: string;
    authMethod: AuthMethod;
    keyId?: string;
  };
}

export function SessionDialog({ open, onOpenChange, editingSession }: SessionDialogProps) {
  const { addSession, updateSession, groups } = useSessionStore();
  const { keys } = useKeyStore();
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Form state
  const [name, setName] = useState(editingSession?.name ?? '');
  const [host, setHost] = useState(editingSession?.host ?? '');
  const [port, setPort] = useState(editingSession?.port ?? 22);
  const [username, setUsername] = useState(editingSession?.username ?? '');
  const [authMethod, setAuthMethod] = useState<AuthMethod>(
    editingSession?.authMethod ?? 'password'
  );
  const [keyId, setKeyId] = useState(editingSession?.keyId ?? '');
  const [group, setGroup] = useState(editingSession?.group ?? '');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    try {
      const sessionData = {
        name,
        host,
        port,
        username,
        authMethod,
        keyId: authMethod === 'key' ? keyId : undefined,
        group: group || undefined,
      };

      if (editingSession) {
        updateSession(editingSession.id, sessionData);
      } else {
        await addSession(sessionData);
      }
      onOpenChange(false);
      // Reset form
      setName('');
      setHost('');
      setPort(22);
      setUsername('');
      setAuthMethod('password');
      setKeyId('');
      setGroup('');
    } catch (error) {
      console.error('Failed to save session:', error);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleOpenChange = (open: boolean) => {
    onOpenChange(open);
    if (!open) {
      // Reset form when closing
      setName('');
      setHost('');
      setPort(22);
      setUsername('');
      setAuthMethod('password');
      setKeyId('');
      setGroup('');
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{editingSession ? 'Edit Connection' : 'New Connection'}</DialogTitle>
          <DialogDescription>
            {editingSession
              ? 'Modify the SSH connection settings.'
              : 'Add a new SSH connection to your sessions.'}
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div className="col-span-2 space-y-2">
              <Label htmlFor="name">Name</Label>
              <Input
                id="name"
                placeholder="e.g., Production Server"
                value={name}
                onChange={(e) => {
                  setName(e.target.value);
                }}
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="host">Host</Label>
              <Input
                id="host"
                placeholder="192.168.1.100"
                value={host}
                onChange={(e) => {
                  setHost(e.target.value);
                }}
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="port">Port</Label>
              <Input
                id="port"
                type="number"
                min={1}
                max={65535}
                value={port}
                onChange={(e) => {
                  setPort(parseInt(e.target.value, 10));
                }}
                required
              />
            </div>

            <div className="col-span-2 space-y-2">
              <Label htmlFor="username">Username</Label>
              <Input
                id="username"
                placeholder="root"
                value={username}
                onChange={(e) => {
                  setUsername(e.target.value);
                }}
                required
              />
            </div>

            <div className="col-span-2 space-y-2">
              <Label htmlFor="authMethod">Authentication</Label>
              <Select
                value={authMethod}
                onValueChange={(value) => {
                  setAuthMethod(value as AuthMethod);
                }}
              >
                <SelectTrigger id="authMethod">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="password">
                    <div className="flex items-center gap-2">
                      <Lock className="h-4 w-4" />
                      Password
                    </div>
                  </SelectItem>
                  <SelectItem value="key">
                    <div className="flex items-center gap-2">
                      <Key className="h-4 w-4" />
                      SSH Key
                    </div>
                  </SelectItem>
                  <SelectItem value="agent">
                    <div className="flex items-center gap-2">
                      <Key className="h-4 w-4" />
                      SSH Agent
                    </div>
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>

            {authMethod === 'key' && (
              <div className="col-span-2 space-y-2">
                <Label htmlFor="keyId">Select Key</Label>
                <Select value={keyId} onValueChange={setKeyId}>
                  <SelectTrigger id="keyId">
                    <SelectValue placeholder="Select a key" />
                  </SelectTrigger>
                  <SelectContent>
                    {keys.length === 0 ? (
                      <SelectItem value="" disabled>
                        No keys available
                      </SelectItem>
                    ) : (
                      keys.map((key) => (
                        <SelectItem key={key.id} value={key.id}>
                          {key.name} ({key.type})
                        </SelectItem>
                      ))
                    )}
                  </SelectContent>
                </Select>
              </div>
            )}

            <div className="col-span-2 space-y-2">
              <Label htmlFor="group">Group (Optional)</Label>
              <Select value={group} onValueChange={setGroup}>
                <SelectTrigger id="group">
                  <SelectValue placeholder="Select a group" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="">No group</SelectItem>
                  {groups.map((grp) => (
                    <SelectItem key={grp.id} value={grp.name}>
                      <div className="flex items-center gap-2">
                        <span
                          className="h-2 w-2 rounded-full"
                          style={{ backgroundColor: grp.color }}
                        />
                        {grp.name}
                      </div>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => {
                handleOpenChange(false);
              }}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? 'Saving...' : editingSession ? 'Save' : 'Create'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
