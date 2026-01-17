import { cn } from '@/lib/utils';
import { useSessionStore } from '@/lib/stores';
import type { SessionStatus } from '@/lib/types';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Search, Server } from 'lucide-react';
import { useState, useMemo } from 'react';

const statusColors: Record<SessionStatus, { bg: string; text: string; dot: string }> = {
  Disconnected: { bg: 'bg-muted', text: 'text-muted-foreground', dot: 'bg-muted-foreground' },
  Connecting: { bg: 'bg-warning/20', text: 'text-warning', dot: 'bg-warning' },
  Connected: { bg: 'bg-success/20', text: 'text-success', dot: 'bg-success' },
  Error: { bg: 'bg-destructive/20', text: 'text-destructive', dot: 'bg-destructive' },
};

interface SessionListProps {
  onSessionClick: (id: string) => void;
}

export function SessionList({ onSessionClick }: SessionListProps) {
  const { sessions, groups, activeSessionId } = useSessionStore();
  const [search, setSearch] = useState('');

  const filteredSessions = useMemo(() => {
    if (!search) return sessions;
    const query = search.toLowerCase();
    return sessions.filter(
      (s) =>
        s.name.toLowerCase().includes(query) ||
        s.host.toLowerCase().includes(query) ||
        s.group?.toLowerCase().includes(query)
    );
  }, [sessions, search]);

  const groupedSessions = useMemo(() => {
    const grouped: Record<string, typeof sessions> = {};
    filteredSessions.forEach((session) => {
      const key = session.group ?? 'ungrouped';
      (grouped[key] ??= []).push(session);
    });
    return grouped;
  }, [filteredSessions]);

  return (
    <div className="flex h-full flex-col">
      {/* Search */}
      <div className="p-3">
        <div className="relative">
          <Search className="text-muted-foreground absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2" />
          <Input
            placeholder="Search sessions..."
            value={search}
            onChange={(e) => {
              setSearch(e.target.value);
            }}
            className="bg-background h-9 pl-9"
          />
        </div>
      </div>

      {/* Session List */}
      <ScrollArea className="flex-1">
        <div className="px-2 pb-2">
          {Object.entries(groupedSessions).map(([groupKey, groupSessions]) => {
            const groupData = groups.find((g) => g.name === groupKey);

            return (
              <div key={groupKey} className="mb-4">
                {/* Group Header */}
                {groupKey !== 'ungrouped' && (
                  <div className="mb-2 flex items-center gap-2 px-2">
                    {groupData?.color && (
                      <span
                        className="h-2 w-2 rounded-full"
                        style={{ backgroundColor: groupData.color }}
                      />
                    )}
                    <span className="text-muted-foreground text-xs font-semibold uppercase">
                      {groupKey}
                    </span>
                    <span className="text-muted-foreground text-xs">{groupSessions.length}</span>
                  </div>
                )}

                {/* Sessions */}
                <div className="space-y-1">
                  {groupSessions.map((session) => {
                    const isActive = session.id === activeSessionId;
                    const statusStyle = statusColors[session.status];

                    return (
                      <button
                        key={session.id}
                        onClick={() => {
                          onSessionClick(session.id);
                        }}
                        className={cn(
                          'flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left transition-colors',
                          isActive ? 'bg-primary/20 text-primary' : 'hover:bg-border'
                        )}
                      >
                        {/* Status Dot */}
                        <span className={cn('h-2 w-2 shrink-0 rounded-full', statusStyle.dot)} />

                        {/* Session Info */}
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center gap-2">
                            <span
                              className={cn(
                                'truncate text-sm font-medium',
                                isActive ? 'text-primary' : 'text-foreground'
                              )}
                            >
                              {session.name}
                            </span>
                            {session.status === 'Connected' && (
                              <Badge
                                variant="secondary"
                                className={cn('text-xs', statusStyle.bg, statusStyle.text)}
                              >
                                Active
                              </Badge>
                            )}
                          </div>
                          <div className="text-muted-foreground flex items-center gap-1 text-xs">
                            <Server className="h-3 w-3" />
                            <span className="truncate">
                              {session.username}@{session.host}:{session.port}
                            </span>
                          </div>
                        </div>
                      </button>
                    );
                  })}
                </div>
              </div>
            );
          })}

          {filteredSessions.length === 0 && (
            <div className="text-muted-foreground py-8 text-center text-sm">
              {search ? 'No sessions found' : 'No sessions yet'}
            </div>
          )}
        </div>
      </ScrollArea>
    </div>
  );
}
