import { createContext, useContext, useState, useCallback, ReactNode } from "react";

interface EnvLoadingState {
  isLoading: boolean;
  message: string;
}

interface AppStateContextType {
  envLoading: EnvLoadingState;
  setEnvLoading: (state: EnvLoadingState) => void;
  envNeedsRefresh: boolean;
  setEnvNeedsRefresh: (value: boolean) => void;
}

const defaultEnvLoading: EnvLoadingState = {
  isLoading: false,
  message: "",
};

const AppStateContext = createContext<AppStateContextType | undefined>(undefined);

export function AppStateProvider({ children }: { children: ReactNode }) {
  const [envLoading, setEnvLoadingState] = useState<EnvLoadingState>(defaultEnvLoading);
  const [envNeedsRefresh, setEnvNeedsRefreshState] = useState(false);

  const setEnvLoading = useCallback((state: EnvLoadingState) => {
    setEnvLoadingState(state);
  }, []);

  const setEnvNeedsRefresh = useCallback((value: boolean) => {
    setEnvNeedsRefreshState(value);
  }, []);

  return (
    <AppStateContext.Provider value={{ envLoading, setEnvLoading, envNeedsRefresh, setEnvNeedsRefresh }}>
      {children}
    </AppStateContext.Provider>
  );
}

export function useAppState() {
  const context = useContext(AppStateContext);
  if (!context) {
    throw new Error("useAppState must be used within AppStateProvider");
  }
  return context;
}
