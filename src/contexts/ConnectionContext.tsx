import { createContext, useContext, useState, useEffect, ReactNode, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface DbConnection {
  id: string;
  name: string;
  host: string;
  port: string;
  database: string;
  user: string;
  password: string;
}

interface ConnectionContextType {
  connections: DbConnection[];
  selectedConnection: DbConnection | null;
  selectConnection: (id: string) => void;
  loadConnections: () => Promise<void>;
  refreshConnections: () => Promise<void>;
}

const ConnectionContext = createContext<ConnectionContextType | undefined>(undefined);

export function ConnectionProvider({ children }: { children: ReactNode }) {
  const [connections, setConnections] = useState<DbConnection[]>([]);
  const [selectedConnection, setSelectedConnection] = useState<DbConnection | null>(null);

  const loadConnections = useCallback(async () => {
    try {
      const conns = await invoke<DbConnection[]>("load_connections");
      setConnections(conns);
      if (conns.length > 0 && !selectedConnection) {
        setSelectedConnection(conns[0]);
      }
    } catch (e) {
      console.error("加载连接失败:", e);
    }
  }, [selectedConnection]);

  const refreshConnections = useCallback(async () => {
    await loadConnections();
  }, [loadConnections]);

  const selectConnection = useCallback((id: string) => {
    const conn = connections.find((c) => c.id === id);
    setSelectedConnection(conn || null);
  }, [connections]);

  useEffect(() => {
    loadConnections();
  }, []);

  useEffect(() => {
    if (connections.length > 0 && !selectedConnection) {
      setSelectedConnection(connections[0]);
    }
  }, [connections, selectedConnection]);

  return (
    <ConnectionContext.Provider
      value={{
        connections,
        selectedConnection,
        selectConnection,
        loadConnections,
        refreshConnections,
      }}
    >
      {children}
    </ConnectionContext.Provider>
  );
}

export function useConnection() {
  const context = useContext(ConnectionContext);
  if (!context) {
    throw new Error("useConnection must be used within ConnectionProvider");
  }
  return context;
}
