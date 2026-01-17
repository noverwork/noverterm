import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';
import { invoke } from '@tauri-apps/api/core';
import type {
  SSHSession,
  CreateSessionInput,
  SessionGroup,
  SSHKey,
  CreateKeyInput,
  ImportKeyInput,
  PortForward,
  CreatePortForwardInput,
  ViewType,
} from './types';

// ============================================================================
// Session Store
// ============================================================================

interface SessionState {
  sessions: SSHSession[];
  groups: SessionGroup[];
  activeSessionId: string | null;
  currentView: ViewType;

  // Session actions
  addSession: (session: CreateSessionInput) => Promise<void>;
  updateSession: (id: string, updates: Partial<SSHSession>) => void;
  deleteSession: (id: string) => Promise<void>;
  setActiveSession: (id: string | null) => void;
  setCurrentView: (view: ViewType) => void;

  // Connection actions
  connectSession: (id: string, password?: string) => Promise<void>;
  disconnectSession: (id: string) => Promise<void>;

  // Group actions
  addGroup: (name: string, color?: string) => void;
  deleteGroup: (id: string) => void;
}

export const useSessionStore = create<SessionState>()(
  immer((set, get) => ({
    sessions: [
      {
        id: '1',
        name: 'server-prod',
        group: 'Production',
        host: '192.168.1.100',
        port: 22,
        username: 'ubuntu',
        authMethod: 'key',
        keyId: 'key-1',
        status: 'Disconnected',
        rows: 24,
        cols: 80,
        portForwards: [],
      },
      {
        id: '2',
        name: 'server-staging',
        group: 'Production',
        host: '192.168.1.101',
        port: 22,
        username: 'ubuntu',
        authMethod: 'key',
        status: 'Disconnected',
        rows: 24,
        cols: 80,
        portForwards: [],
      },
      {
        id: '3',
        name: 'db-primary',
        group: 'Development',
        host: '192.168.1.200',
        port: 22,
        username: 'root',
        authMethod: 'password',
        status: 'Disconnected',
        rows: 24,
        cols: 80,
        portForwards: [],
      },
    ],
    groups: [
      { id: 'g1', name: 'Production', color: '#ef4444' },
      { id: 'g2', name: 'Development', color: '#3b82f6' },
    ],
    activeSessionId: null,
    currentView: 'dashboard',

    addSession: async (session) => {
      const newSession: SSHSession = {
        id: `session-${Date.now()}`,
        ...session,
        status: 'Disconnected',
        rows: 24,
        cols: 80,
        portForwards: [],
        error: undefined,
      };
      set((state) => {
        state.sessions.push(newSession);
      });

      try {
        await invoke('create_session', { session: newSession });
      } catch (error) {
        console.error('Failed to create session:', error);
      }
    },

    updateSession: (id, updates) => {
      set((state) => {
        const session = state.sessions.find((s) => s.id === id);
        if (session) {
          Object.assign(session, updates);
        }
      });
    },

    deleteSession: async (id) => {
      set((state) => {
        state.sessions = state.sessions.filter((s) => s.id !== id);
        if (state.activeSessionId === id) {
          state.activeSessionId = null;
        }
      });

      try {
        await invoke('delete_session', { sessionId: id });
      } catch (error) {
        console.error('Failed to delete session:', error);
      }
    },

    setActiveSession: (id) => {
      set((state) => {
        state.activeSessionId = id;
        if (id) {
          state.currentView = 'terminal';
        } else {
          state.currentView = 'dashboard';
        }
      });
    },

    setCurrentView: (view) => {
      set((state) => {
        state.currentView = view;
      });
    },

    connectSession: async (id, password) => {
      set((state) => {
        const session = state.sessions.find((s) => s.id === id);
        if (session) {
          session.status = 'Connecting';
          session.error = undefined;
        }
      });

      try {
        await invoke('connect_session', { sessionId: id, password });
        set((state) => {
          const session = state.sessions.find((s) => s.id === id);
          if (session) {
            session.status = 'Connected';
          }
        });
        get().setActiveSession(id);
      } catch (error) {
        set((state) => {
          const session = state.sessions.find((s) => s.id === id);
          if (session) {
            session.status = 'Error';
            session.error = error instanceof Error ? error.message : 'Connection failed';
          }
        });
      }
    },

    disconnectSession: async (id) => {
      try {
        await invoke('disconnect_session', { sessionId: id });
      } catch (error) {
        console.error('Failed to disconnect:', error);
      }

      set((state) => {
        const session = state.sessions.find((s) => s.id === id);
        if (session) {
          session.status = 'Disconnected';
        }
      });
    },

    addGroup: (name, color) => {
      set((state) => {
        state.groups.push({
          id: `group-${Date.now()}`,
          name,
          color,
        });
      });
    },

    deleteGroup: (id) => {
      set((state) => {
        state.groups = state.groups.filter((g) => g.id !== id);
        state.sessions.forEach((s) => {
          if (s.group === state.groups.find((g) => g.id === id)?.name) {
            s.group = undefined;
          }
        });
      });
    },
  }))
);

// ============================================================================
// Key Store
// ============================================================================

interface KeyState {
  keys: SSHKey[];
  selectedKeyId: string | null;

  // Key actions
  addKey: (key: CreateKeyInput) => Promise<void>;
  importKey: (key: ImportKeyInput) => Promise<void>;
  deleteKey: (id: string) => Promise<void>;
  setSelectedKey: (id: string | null) => void;
}

export const useKeyStore = create<KeyState>()(
  immer((set) => ({
    keys: [],
    selectedKeyId: null,

    addKey: async (key) => {
      try {
        const result = await invoke<string>('generate_ssh_key', {
          name: key.name,
          type: key.type,
          passphrase: key.passphrase,
        });

        const newKey: SSHKey = {
          id: `key-${Date.now()}`,
          name: key.name,
          type: key.type,
          publicKey: result,
          fingerprint: '',
          createdAt: new Date().toISOString(),
          hasPassphrase: !!key.passphrase,
        };

        set((state) => {
          state.keys.push(newKey);
        });
      } catch (error) {
        console.error('Failed to generate key:', error);
        throw error;
      }
    },

    importKey: async (key) => {
      try {
        await invoke('import_ssh_key', {
          path: key.privateKeyPath,
          name: key.name,
        });

        const newKey: SSHKey = {
          id: `key-${Date.now()}`,
          name: key.name,
          type: 'rsa',
          publicKey: key.publicKey,
          privateKeyPath: key.privateKeyPath,
          fingerprint: '',
          createdAt: new Date().toISOString(),
          hasPassphrase: false,
        };

        set((state) => {
          state.keys.push(newKey);
        });
      } catch (error) {
        console.error('Failed to import key:', error);
        throw error;
      }
    },

    deleteKey: async (id) => {
      set((state) => {
        state.keys = state.keys.filter((k) => k.id !== id);
      });

      try {
        await invoke('delete_ssh_key', { keyId: id });
      } catch (error) {
        console.error('Failed to delete key:', error);
      }
    },

    setSelectedKey: (id) => {
      set((state) => {
        state.selectedKeyId = id;
      });
    },
  }))
);

// ============================================================================
// Port Forward Store
// ============================================================================

interface PortForwardState {
  portForwards: PortForward[];

  // Port forward actions
  addPortForward: (forward: CreatePortForwardInput) => Promise<void>;
  removePortForward: (id: string) => Promise<void>;
  togglePortForward: (id: string) => Promise<void>;
  getPortForwardsBySession: (sessionId: string) => PortForward[];
}

export const usePortForwardStore = create<PortForwardState>()(
  immer((set, get) => ({
    portForwards: [],

    addPortForward: async (forward) => {
      const newForward: PortForward = {
        id: `pf-${Date.now()}`,
        ...forward,
        active: false,
      };

      set((state) => {
        state.portForwards.push(newForward);
      });

      try {
        await invoke('add_port_forward', { config: newForward });
      } catch (error) {
        console.error('Failed to add port forward:', error);
      }
    },

    removePortForward: async (id) => {
      set((state) => {
        state.portForwards = state.portForwards.filter((pf) => pf.id !== id);
      });

      try {
        await invoke('remove_port_forward', { forwardId: id });
      } catch (error) {
        console.error('Failed to remove port forward:', error);
      }
    },

    togglePortForward: async (id) => {
      const forward = get().portForwards.find((pf) => pf.id === id);
      if (!forward) return;

      const newActive = !forward.active;

      set((state) => {
        const pf = state.portForwards.find((p) => p.id === id);
        if (pf) {
          pf.active = newActive;
        }
      });

      try {
        await invoke('toggle_port_forward', {
          forwardId: id,
          active: newActive,
        });
      } catch (error) {
        console.error('Failed to toggle port forward:', error);
        // Revert on error
        set((state) => {
          const pf = state.portForwards.find((p) => p.id === id);
          if (pf) {
            pf.active = !newActive;
          }
        });
      }
    },

    getPortForwardsBySession: (sessionId) => {
      return get().portForwards.filter((pf) => pf.sessionId === sessionId);
    },
  }))
);
