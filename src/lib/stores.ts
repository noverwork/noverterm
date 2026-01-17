import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';
import { invoke } from '@tauri-apps/api/core';
import { db } from './db';
import {
  mapDbSessionToUi,
  mapUiSessionToDbInput,
  mapDbGroupToUi,
  mapDbKeyToUi,
  mapUiKeyToDbInput,
  mapUiImportKeyToDbInput,
  mapDbPortForwardToUi,
  mapUiPortForwardToDbInput,
} from './mappers';
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
  isInitialized: boolean;

  // Initialization
  init: () => Promise<void>;

  // Session actions
  addSession: (session: CreateSessionInput, groupId?: string) => Promise<void>;
  updateSession: (id: string, updates: Partial<SSHSession>) => Promise<void>;
  deleteSession: (id: string) => Promise<void>;
  setActiveSession: (id: string | null) => void;
  setCurrentView: (view: ViewType) => void;

  // Connection actions
  connectSession: (id: string, password?: string) => Promise<void>;
  disconnectSession: (id: string) => Promise<void>;

  // Group actions
  addGroup: (name: string, color?: string) => Promise<void>;
  updateGroup: (id: string, name?: string, color?: string) => Promise<void>;
  deleteGroup: (id: string) => Promise<void>;

  // Refresh data from DB
  refreshSessions: () => Promise<void>;
  refreshGroups: () => Promise<void>;
}

export const useSessionStore = create<SessionState>()(
  immer((set, get) => ({
    sessions: [],
    groups: [],
    activeSessionId: null,
    currentView: 'dashboard',
    isInitialized: false,

    init: async () => {
      if (get().isInitialized) return;

      try {
        const [dbSessions, dbGroups] = await Promise.all([
          db.getSessions(),
          db.getGroups(),
        ]);

        const groups = dbGroups.map(mapDbGroupToUi);
        const groupMap = new Map(groups.map(g => [g.id, g.name]));

        // Get port forwards for each session
        const sessions = await Promise.all(
          dbSessions.map(async (dbSession) => {
            const portForwards = await db.getPortForwards(dbSession.id);
            const uiSession = mapDbSessionToUi(dbSession, portForwards.map(mapDbPortForwardToUi));
            // Resolve group_id to group name
            if (dbSession.group_id) {
              uiSession.group = groupMap.get(dbSession.group_id);
            }
            return uiSession;
          })
        );

        set({ sessions, groups, isInitialized: true });
      } catch (error) {
        console.error('Failed to initialize store:', error);
      }
    },

    addSession: async (session, groupId) => {
      try {
        const dbInput = mapUiSessionToDbInput(session, groupId);
        const dbSession = await db.createSession(dbInput);
        const uiSession = mapDbSessionToUi(dbSession);

        // Resolve group name
        if (groupId) {
          const group = get().groups.find(g => g.id === groupId);
          if (group) uiSession.group = group.name;
        }

        set((state) => {
          state.sessions.push(uiSession);
        });
      } catch (error) {
        console.error('Failed to create session:', error);
        throw error;
      }
    },

    updateSession: async (id, updates) => {
      try {
        // Map UI updates to DB updates
        const dbInput: Record<string, unknown> = {};
        if (updates.name !== undefined) dbInput.name = updates.name;
        if (updates.host !== undefined) dbInput.host = updates.host;
        if (updates.port !== undefined) dbInput.port = updates.port;
        if (updates.username !== undefined) dbInput.username = updates.username;
        if (updates.authMethod !== undefined) dbInput.auth_method = updates.authMethod;
        if (updates.keyId !== undefined) dbInput.key_id = updates.keyId;

        const dbSession = await db.updateSession(id, dbInput);

        set((state) => {
          const session = state.sessions.find((s) => s.id === id);
          if (session) {
            Object.assign(session, mapDbSessionToUi(dbSession));
            // Preserve runtime state
            session.status = updates.status ?? session.status;
            session.error = updates.error ?? session.error;
          }
        });
      } catch (error) {
        console.error('Failed to update session:', error);
        throw error;
      }
    },

    deleteSession: async (id) => {
      try {
        await db.deleteSession(id);

        set((state) => {
          state.sessions = state.sessions.filter((s) => s.id !== id);
          if (state.activeSessionId === id) {
            state.activeSessionId = null;
          }
        });
      } catch (error) {
        console.error('Failed to delete session:', error);
        throw error;
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
        throw error;
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

    addGroup: async (name, color) => {
      try {
        const dbGroup = await db.createGroup({ name, color });
        set((state) => {
          state.groups.push(mapDbGroupToUi(dbGroup));
        });
      } catch (error) {
        console.error('Failed to create group:', error);
        throw error;
      }
    },

    updateGroup: async (id, name, color) => {
      try {
        const dbGroup = await db.updateGroup(id, { name, color });
        set((state) => {
          const idx = state.groups.findIndex((g) => g.id === id);
          if (idx !== -1) {
            state.groups[idx] = mapDbGroupToUi(dbGroup);
          }
        });
      } catch (error) {
        console.error('Failed to update group:', error);
        throw error;
      }
    },

    deleteGroup: async (id) => {
      try {
        await db.deleteGroup(id);
        set((state) => {
          state.groups = state.groups.filter((g) => g.id !== id);
          // Clear group reference in sessions
          state.sessions.forEach((s) => {
            if (s.group === state.groups.find((g) => g.id === id)?.name) {
              s.group = undefined;
            }
          });
        });
      } catch (error) {
        console.error('Failed to delete group:', error);
        throw error;
      }
    },

    refreshSessions: async () => {
      try {
        const dbSessions = await db.getSessions();
        const groupMap = new Map(get().groups.map(g => [g.id, g.name]));

        const sessions = await Promise.all(
          dbSessions.map(async (dbSession) => {
            const portForwards = await db.getPortForwards(dbSession.id);
            const uiSession = mapDbSessionToUi(dbSession, portForwards.map(mapDbPortForwardToUi));
            if (dbSession.group_id) {
              uiSession.group = groupMap.get(dbSession.group_id);
            }
            // Preserve runtime state
            const existing = get().sessions.find(s => s.id === dbSession.id);
            if (existing) {
              uiSession.status = existing.status;
              uiSession.error = existing.error;
              uiSession.rows = existing.rows;
              uiSession.cols = existing.cols;
            }
            return uiSession;
          })
        );

        set({ sessions });
      } catch (error) {
        console.error('Failed to refresh sessions:', error);
      }
    },

    refreshGroups: async () => {
      try {
        const dbGroups = await db.getGroups();
        const groups = dbGroups.map(mapDbGroupToUi);
        set({ groups });
      } catch (error) {
        console.error('Failed to refresh groups:', error);
      }
    },
  }))
);

// ============================================================================
// Key Store
// ============================================================================

interface KeyState {
  keys: SSHKey[];
  selectedKeyId: string | null;
  isInitialized: boolean;

  // Initialization
  init: () => Promise<void>;

  // Key actions
  addKey: (key: CreateKeyInput) => Promise<void>;
  importKey: (key: ImportKeyInput, fingerprint: string, hasPassphrase: boolean) => Promise<void>;
  updateKey: (id: string, name: string) => Promise<void>;
  deleteKey: (id: string) => Promise<void>;
  setSelectedKey: (id: string | null) => void;

  // Refresh
  refreshKeys: () => Promise<void>;
}

export const useKeyStore = create<KeyState>()(
  immer((set, get) => ({
    keys: [],
    selectedKeyId: null,
    isInitialized: false,

    init: async () => {
      if (get().isInitialized) return;

      try {
        const dbKeys = await db.getKeys();
        const keys = dbKeys.map(mapDbKeyToUi);
        set({ keys, isInitialized: true });
      } catch (error) {
        console.error('Failed to initialize key store:', error);
      }
    },

    addKey: async (key) => {
      try {
        const result = await invoke<string>('generate_ssh_key', {
          name: key.name,
          type: key.type,
          passphrase: key.passphrase,
        });

        const fingerprint = await invoke<string>('get_key_fingerprint', { publicKey: result });

        const dbInput = mapUiKeyToDbInput(key, result, fingerprint);
        const dbKey = await db.createKey(dbInput);

        set((state) => {
          state.keys.push(mapDbKeyToUi(dbKey));
        });
      } catch (error) {
        console.error('Failed to generate key:', error);
        throw error;
      }
    },

    importKey: async (key, fingerprint, hasPassphrase) => {
      try {
        await invoke('import_ssh_key', {
          path: key.privateKeyPath,
          name: key.name,
        });

        const dbInput = mapUiImportKeyToDbInput(key, fingerprint, hasPassphrase);
        const dbKey = await db.createKey(dbInput);

        set((state) => {
          state.keys.push(mapDbKeyToUi(dbKey));
        });
      } catch (error) {
        console.error('Failed to import key:', error);
        throw error;
      }
    },

    updateKey: async (id, name) => {
      try {
        const dbKey = await db.updateKey(id, { name });
        set((state) => {
          const idx = state.keys.findIndex((k) => k.id === id);
          if (idx !== -1) {
            state.keys[idx] = mapDbKeyToUi(dbKey);
          }
        });
      } catch (error) {
        console.error('Failed to update key:', error);
        throw error;
      }
    },

    deleteKey: async (id) => {
      try {
        await db.deleteKey(id);
        set((state) => {
          state.keys = state.keys.filter((k) => k.id !== id);
        });
      } catch (error) {
        console.error('Failed to delete key:', error);
        throw error;
      }
    },

    setSelectedKey: (id) => {
      set((state) => {
        state.selectedKeyId = id;
      });
    },

    refreshKeys: async () => {
      try {
        const dbKeys = await db.getKeys();
        const keys = dbKeys.map(mapDbKeyToUi);
        set({ keys });
      } catch (error) {
        console.error('Failed to refresh keys:', error);
      }
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
  updatePortForward: (id: string, updates: Partial<PortForward>) => Promise<void>;
  togglePortForward: (id: string) => Promise<void>;
  getPortForwardsBySession: (sessionId: string) => PortForward[];

  // Refresh
  refreshPortForwards: (sessionId?: string) => Promise<void>;
}

export const usePortForwardStore = create<PortForwardState>()(
  immer((set, get) => ({
    portForwards: [],

    addPortForward: async (forward) => {
      try {
        const dbInput = mapUiPortForwardToDbInput(forward);
        const dbPortForward = await db.createPortForward(dbInput);
        const uiPortForward = mapDbPortForwardToUi(dbPortForward);

        set((state) => {
          state.portForwards.push(uiPortForward);
        });

        // Also try to activate via the backend
        try {
          await invoke('add_port_forward', { config: uiPortForward });
        } catch (error) {
          console.error('Failed to activate port forward:', error);
        }
      } catch (error) {
        console.error('Failed to add port forward:', error);
        throw error;
      }
    },

    removePortForward: async (id) => {
      try {
        await db.deletePortForward(id);

        set((state) => {
          state.portForwards = state.portForwards.filter((pf) => pf.id !== id);
        });

        try {
          await invoke('remove_port_forward', { forwardId: id });
        } catch (error) {
          console.error('Failed to remove port forward from backend:', error);
        }
      } catch (error) {
        console.error('Failed to remove port forward:', error);
        throw error;
      }
    },

    updatePortForward: async (id, updates) => {
      try {
        const dbInput: Record<string, unknown> = {};
        if (updates.name !== undefined) dbInput.name = updates.name;
        if (updates.type !== undefined) dbInput.type = updates.type;
        if (updates.localHost !== undefined) dbInput.local_host = updates.localHost;
        if (updates.localPort !== undefined) dbInput.local_port = updates.localPort;
        if (updates.remoteHost !== undefined) dbInput.remote_host = updates.remoteHost;
        if (updates.remotePort !== undefined) dbInput.remote_port = updates.remotePort;

        const dbPortForward = await db.updatePortForward(id, dbInput);

        set((state) => {
          const idx = state.portForwards.findIndex((pf) => pf.id === id);
          if (idx !== -1) {
            const uiPortForward = mapDbPortForwardToUi(dbPortForward);
            uiPortForward.active = state.portForwards[idx].active; // Preserve active state
            state.portForwards[idx] = uiPortForward;
          }
        });
      } catch (error) {
        console.error('Failed to update port forward:', error);
        throw error;
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
        throw error;
      }
    },

    getPortForwardsBySession: (sessionId) => {
      return get().portForwards.filter((pf) => pf.sessionId === sessionId);
    },

    refreshPortForwards: async (sessionId) => {
      try {
        const dbPortForwards = await db.getPortForwards(sessionId);
        const portForwards = dbPortForwards.map(mapDbPortForwardToUi);

        // Preserve active state for existing forwards
        const existingMap = new Map(get().portForwards.map(pf => [pf.id, pf.active]));

        set((state) => {
          state.portForwards = portForwards.map(pf => ({
            ...pf,
            active: existingMap.get(pf.id) ?? false,
          }));
        });
      } catch (error) {
        console.error('Failed to refresh port forwards:', error);
      }
    },
  }))
);
