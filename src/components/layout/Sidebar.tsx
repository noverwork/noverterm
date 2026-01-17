import { cn } from '@/lib/utils';
import { useSessionStore } from '@/lib/stores';
import { Terminal, Key, ArrowRightLeft, Folder, Plus, Settings } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import type { ViewType } from '@/lib/types';

const navItems = [
  { id: 'dashboard' as ViewType, label: 'Sessions', icon: Terminal },
  { id: 'keys' as ViewType, label: 'Keys', icon: Key },
  { id: 'portForwards' as ViewType, label: 'Port Forwards', icon: ArrowRightLeft },
];

interface SidebarProps {
  onNewSession: () => void;
}

export function Sidebar({ onNewSession }: SidebarProps) {
  const { currentView, setCurrentView, sessions, groups } = useSessionStore();

  const activeSessions = sessions.filter((s) => s.status === 'Connected').length;

  return (
    <div className="border-border bg-surface flex h-full w-64 flex-col border-r">
      {/* Header */}
      <div className="border-border flex h-14 items-center border-b px-4">
        <Terminal className="text-primary mr-2 h-5 w-5" />
        <span className="text-foreground text-lg font-bold">Noverterm</span>
      </div>

      {/* New Connection Button */}
      <div className="p-3">
        <Button
          onClick={onNewSession}
          className="bg-primary text-primary-foreground hover:bg-primary/90 h-10 w-full justify-start"
        >
          <Plus className="mr-2 h-4 w-4" />
          New Connection
        </Button>
      </div>

      <Separator />

      {/* Navigation */}
      <ScrollArea className="flex-1">
        <div className="space-y-1 p-2">
          {navItems.map((item) => {
            const Icon = item.icon;
            const isActive = currentView === item.id;
            const badge = item.id === 'dashboard' ? activeSessions : undefined;

            return (
              <button
                key={item.id}
                onClick={() => {
                  setCurrentView(item.id);
                }}
                className={cn(
                  'flex w-full items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors',
                  isActive
                    ? 'bg-primary/20 text-primary'
                    : 'text-muted-foreground hover:bg-border hover:text-foreground'
                )}
              >
                <Icon className="h-4 w-4" />
                <span className="flex-1 text-left">{item.label}</span>
                {badge !== undefined && badge > 0 && (
                  <span
                    className={cn(
                      'flex h-5 min-w-5 items-center justify-center rounded-full px-1.5 text-xs',
                      isActive
                        ? 'bg-primary text-primary-foreground'
                        : 'bg-muted text-muted-foreground'
                    )}
                  >
                    {badge}
                  </span>
                )}
              </button>
            );
          })}
        </div>

        {/* Groups */}
        {groups.length > 0 && currentView === 'dashboard' && (
          <>
            <Separator className="my-2" />
            <div className="px-3 py-2">
              <div className="text-muted-foreground mb-2 flex items-center gap-2 text-xs font-semibold uppercase">
                <Folder className="h-3 w-3" />
                Groups
              </div>
              <div className="space-y-1">
                {groups.map((group) => (
                  <button
                    key={group.id}
                    className="text-muted-foreground hover:bg-border hover:text-foreground flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm transition-colors"
                  >
                    <span
                      className="h-2 w-2 rounded-full"
                      style={{ backgroundColor: group.color }}
                    />
                    <span>{group.name}</span>
                    <span className="ml-auto text-xs">
                      {sessions.filter((s) => s.group === group.name).length}
                    </span>
                  </button>
                ))}
              </div>
            </div>
          </>
        )}
      </ScrollArea>

      <Separator />

      {/* Footer */}
      <div className="p-2">
        <button
          onClick={() => {
            setCurrentView('dashboard');
          }}
          className={cn(
            'flex w-full items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors',
            'text-muted-foreground hover:bg-border hover:text-foreground'
          )}
        >
          <Settings className="h-4 w-4" />
          <span>Settings</span>
        </button>
      </div>
    </div>
  );
}
