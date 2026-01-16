import { useState } from 'react';
import { Sidebar } from '@/components/Sidebar';
import { TerminalCanvas } from '@/components/TerminalCanvas';
import type { SSHSession } from '@/components/SSHCard';
import { SSHCard } from '@/components/SSHCard';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { X, Plus } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { cn } from '@/lib/utils';

function App() {
  const [sessions, setSessions] = useState<SSHSession[]>([
    {
      id: '1',
      name: 'server-01',
      host: '192.168.1.100',
      port: 22,
      username: 'ubuntu',
      status: 'Disconnected',
    },
    {
      id: '2',
      name: 'server-02',
      host: '192.168.1.101',
      port: 22,
      username: 'root',
      status: 'Connected',
    },
  ]);
  const [activeSession, setActiveSession] = useState<string | null>('2');

  const handleConnect = async (id: string) => {
    setSessions((sessions) =>
      sessions.map((s) => (s.id === id ? { ...s, status: 'Connecting' } : s))
    );

    // Call Rust command
    try {
      await invoke('connect_session', { sessionId: id });
      setSessions((sessions) =>
        sessions.map((s) => (s.id === id ? { ...s, status: 'Connected' } : s))
      );
    } catch (error) {
      console.error('Connection failed:', error);
      setSessions((sessions) => sessions.map((s) => (s.id === id ? { ...s, status: 'Error' } : s)));
    }
  };

  const handleDisconnect = async (id: string) => {
    try {
      await invoke('disconnect_session', { sessionId: id });
    } catch (error) {
      console.error('Disconnect failed:', error);
    }

    setSessions((sessions) =>
      sessions.map((s) => (s.id === id ? { ...s, status: 'Disconnected' } : s))
    );

    if (activeSession === id) {
      setActiveSession(null);
    }
  };

  const handleDelete = (id: string) => {
    setSessions((sessions) => sessions.filter((s) => s.id !== id));
    if (activeSession === id) {
      setActiveSession(null);
    }
  };

  return (
    <div className="flex h-screen bg-slate-950 font-sans text-slate-100">
      <Sidebar />

      {/* Main Content */}
      <div className="flex flex-1 flex-col overflow-hidden">
        {/* Top Bar */}
        <header className="flex items-center justify-between border-b border-slate-800 bg-slate-900/50 px-6 py-3">
          <div className="flex items-center gap-4">
            <h2 className="text-lg font-semibold">Terminal</h2>
            {activeSession && (
              <span className="text-sm text-slate-400">
                {sessions.find((s) => s.id === activeSession)?.name}
              </span>
            )}
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" className="h-8 border-slate-700">
              Split
            </Button>
            <Button variant="outline" size="sm" className="h-8 border-slate-700">
              New Tab
            </Button>
          </div>
        </header>

        {/* Terminal Area */}
        <div className="flex-1 overflow-auto p-4">
          {activeSession ? (
            <div className="h-full">
              <TerminalCanvas sessionId={activeSession} className="h-full" />
            </div>
          ) : (
            // Empty State - Show SSH Cards
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
              {sessions.map((session) => (
                <div
                  key={session.id}
                  className="cursor-pointer"
                  onClick={() => {
                    if (session.status === 'Connected') {
                      setActiveSession(session.id);
                    }
                  }}
                >
                  <SSHCard
                    session={session}
                    onConnect={handleConnect}
                    onDisconnect={handleDisconnect}
                    onDelete={handleDelete}
                  />
                </div>
              ))}
              {/* Add New Connection Card */}
              <Card className="flex min-h-[120px] cursor-pointer items-center justify-center border-dashed border-slate-700/50 bg-slate-800/30 transition-colors hover:border-slate-600">
                <div className="text-center text-slate-500">
                  <Plus className="mx-auto mb-2 h-8 w-8" />
                  <span className="text-sm">Add SSH Connection</span>
                </div>
              </Card>
            </div>
          )}
        </div>
      </div>

      {/* Connection Panel (Right Sidebar) - Optional */}
      <aside className="hidden w-80 border-l border-slate-800 bg-slate-900/30 lg:block">
        <div className="p-4">
          <h3 className="mb-4 text-sm font-semibold text-slate-400">Quick Connect</h3>
          <div className="space-y-2">
            {sessions.map((session) => (
              <button
                key={session.id}
                onClick={() => {
                  if (session.status === 'Connected') {
                    setActiveSession(session.id);
                  }
                }}
                className={cn(
                  'flex w-full items-center justify-between rounded-lg p-3 text-left transition-colors',
                  activeSession === session.id
                    ? 'bg-blue-600/20 text-blue-400'
                    : 'bg-slate-800/50 text-slate-400 hover:bg-slate-800'
                )}
              >
                <div className="flex items-center gap-3">
                  <div
                    className={cn(
                      'h-2 w-2 rounded-full',
                      session.status === 'Connected' ? 'bg-green-500' : 'bg-slate-600'
                    )}
                  />
                  <div>
                    <div className="text-sm font-medium">{session.name}</div>
                    <div className="text-xs text-slate-500">{session.host}</div>
                  </div>
                </div>
                {activeSession === session.id && (
                  <Button variant="ghost" size="icon" className="h-6 w-6 text-slate-400">
                    <X className="h-3 w-3" />
                  </Button>
                )}
              </button>
            ))}
          </div>
        </div>
      </aside>
    </div>
  );
}

export default App;
