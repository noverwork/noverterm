import { useKeyStore } from '@/lib/stores';
import { KeyCard } from './key-card';
import { KeyDialog } from './key-dialog';
import { ScrollArea } from '@/components/ui/scroll-area';
import { EmptyState } from '@/components/ui/empty-state';

export function KeyList() {
  const { keys, deleteKey } = useKeyStore();

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="border-border flex items-center justify-between border-b px-6 py-4">
        <div>
          <h2 className="text-foreground text-lg font-semibold">SSH Keys</h2>
          <p className="text-muted-foreground text-sm">Manage your SSH keys for authentication</p>
        </div>
        <KeyDialog />
      </div>

      {/* Keys Grid */}
      <ScrollArea className="flex-1">
        <div className="p-6">
          {keys.length === 0 ? (
            <EmptyState
              icon={<KeyDialog />}
              title="No SSH keys yet"
              description="Add your first SSH key to authenticate with your servers"
            />
          ) : (
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
              {keys.map((sshKey) => (
                <KeyCard key={sshKey.id} sshKey={sshKey} onDelete={deleteKey} />
              ))}
            </div>
          )}
        </div>
      </ScrollArea>
    </div>
  );
}
