import { useState, useEffect } from 'react';
import { Sidebar } from '@/components/layout/sidebar';
import { TopBar } from '@/components/layout/top-bar';
import { RightPanel } from '@/components/layout/right-panel';
import { SessionDialog } from '@/components/sessions/session-dialog';
import { SessionCard } from '@/components/sessions/session-card';
import { KeyList } from '@/components/keys/key-list';
import { PortForwardList } from '@/components/portforward/port-forward-list';
import { TerminalCanvas } from '@/components/terminal-canvas';
import { useSessionStore, useKeyStore } from '@/lib/stores';
import { Toaster } from '@/components/ui/sonner';

function App() {
  const { currentView, sessions, activeSessionId, connectSession, disconnectSession, init: initSessions } =
    useSessionStore();
  const { init: initKeys } = useKeyStore();

  // Initialize stores from database on app start
  useEffect(() => {
    const initializeApp = async () => {
      try {
        await Promise.all([
          initSessions(),
          initKeys(),
        ]);
      } catch (error) {
        console.error('Failed to initialize app:', error);
      }
    };

    initializeApp();
  }, [initSessions, initKeys]);

  const [sessionDialogOpen, setSessionDialogOpen] = useState(false);
  const [editingSession, setEditingSession] = useState<(typeof sessions)[0] | undefined>();

  const activeSession = sessions.find((s) => s.id === activeSessionId);

  const handleEditSession = (session: (typeof sessions)[0]) => {
    setEditingSession(session);
    setSessionDialogOpen(true);
  };

  const handleNewSession = () => {
    setEditingSession(undefined);
    setSessionDialogOpen(true);
  };

  const renderContent = () => {
    switch (currentView) {
      case 'terminal':
        return activeSessionId ? (
          <div className="flex h-full">
            <div className="flex-1">
              <TerminalCanvas sessionId={activeSessionId} className="h-full" />
            </div>
          </div>
        ) : (
          renderDashboard()
        );

      case 'keys':
        return <KeyList />;

      case 'portForwards':
        return <PortForwardList />;

      case 'dashboard':
      default:
        return renderDashboard();
    }
  };

  const renderDashboard = () => {
    return (
      <div className="flex h-full flex-col">
        {activeSession ? (
          <div className="flex-1">
            <TerminalCanvas sessionId={activeSession.id} className="h-full" />
          </div>
        ) : (
          <div className="flex-1 overflow-auto p-6">
            <div className="mb-6">
              <h2 className="text-foreground text-2xl font-bold">Sessions</h2>
              <p className="text-muted-foreground">
                Connect to your SSH sessions or create a new one
              </p>
            </div>

            {sessions.length === 0 ? (
              <div className="border-border flex flex-col items-center justify-center rounded-lg border border-dashed py-16">
                <p className="text-muted-foreground">
                  No sessions configured. Click "New Connection" to get started.
                </p>
              </div>
            ) : (
              <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
                {sessions.map((session) => (
                  <SessionCard
                    key={session.id}
                    session={session}
                    onConnect={connectSession}
                    onDisconnect={disconnectSession}
                    onEdit={handleEditSession}
                    onDelete={(id) => useSessionStore.getState().deleteSession(id)}
                  />
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="bg-background text-foreground flex h-screen font-sans">
      {/* Left Sidebar */}
      <Sidebar onNewSession={handleNewSession} />

      {/* Main Content Area */}
      <div className="flex flex-1 flex-col overflow-hidden">
        <TopBar />
        {renderContent()}
      </div>

      {/* Right Panel - Shows when session is active */}
      {currentView === 'terminal' && <RightPanel />}

      {/* Session Dialog */}
      <SessionDialog
        open={sessionDialogOpen}
        onOpenChange={(open) => {
          setSessionDialogOpen(open);
          if (!open) setEditingSession(undefined);
        }}
        editingSession={editingSession}
      />

      {/* Toast Notifications */}
      <Toaster
        position="bottom-right"
        toastOptions={{
          duration: 3000,
          classNames: {
            closeButton: 'bg-border hover:bg-border/80',
          },
        }}
      />
    </div>
  );
}

export default App;
