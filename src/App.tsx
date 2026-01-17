import { useState } from 'react';
import { Sidebar } from '@/components/layout/Sidebar';
import { TopBar } from '@/components/layout/TopBar';
import { RightPanel } from '@/components/layout/RightPanel';
import { SessionDialog } from '@/components/sessions/SessionDialog';
import { SessionCard } from '@/components/sessions/SessionCard';
import { KeyList } from '@/components/keys/KeyList';
import { PortForwardList } from '@/components/portforward/PortForwardList';
import { TerminalCanvas } from '@/components/TerminalCanvas';
import { useSessionStore } from '@/lib/stores';
import { Toaster } from '@/components/ui/sonner';

function App() {
  const { currentView, sessions, activeSessionId, connectSession, disconnectSession } =
    useSessionStore();

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
