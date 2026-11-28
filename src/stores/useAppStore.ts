import { create } from "zustand";
import {
  DEFAULT_APP_SETTINGS,
  type ActiveTask,
  type AppSettings,
  type ArchiveInfo,
  type EngineStatus,
  type TaskProgress,
} from "../types/archive";

export interface ToastNotice {
  id: string;
  type: "success" | "error" | "info";
  title: string;
  detail?: string;
  actionPath?: string;
  actionReveal?: boolean;
}

interface AppState {
  engineStatus: EngineStatus | null;
  settings: AppSettings;
  standaloneMode: boolean;
  archive: ArchiveInfo | null;
  loading: boolean;
  encodingOverride: string;
  currentDir: string;
  searchQuery: string;
  selectedPaths: Set<string>;
  activeTask: ActiveTask | null;
  toast: ToastNotice | null;

  // 新建压缩弹窗状态
  compressModalOpen: boolean;
  compressInputs: string[];

  setEngineStatus: (status: EngineStatus) => void;
  setSettings: (settings: AppSettings) => void;
  setStandaloneMode: (standalone: boolean) => void;
  setArchive: (info: ArchiveInfo | null) => void;
  setLoading: (loading: boolean) => void;
  setEncodingOverride: (enc: string) => void;
  setCurrentDir: (dir: string) => void;
  setSearchQuery: (q: string) => void;
  toggleSelect: (path: string) => void;
  clearSelection: () => void;
  selectAll: (paths: string[]) => void;

  // 任务进度弹窗管理
  startTask: (task: ActiveTask) => void;
  updateTaskProgress: (progress: TaskProgress) => void;
  patchActiveTask: (patch: Partial<ActiveTask>) => void;
  clearActiveTask: () => void;

  showToast: (toast: Omit<ToastNotice, "id">) => void;
  clearToast: () => void;
  openCompressModal: (inputs?: string[]) => void;
  closeCompressModal: () => void;
  setCompressInputs: (inputs: string[]) => void;
  resetArchiveView: () => void;
}

export const useAppStore = create<AppState>((set) => ({
  engineStatus: null,
  settings: DEFAULT_APP_SETTINGS,
  standaloneMode: false,
  archive: null,
  loading: false,
  encodingOverride: "auto",
  currentDir: "",
  searchQuery: "",
  selectedPaths: new Set(),
  activeTask: null,
  toast: null,
  compressModalOpen: false,
  compressInputs: [],

  setEngineStatus: (engineStatus) => set({ engineStatus }),
  setSettings: (settings) => set({ settings }),
  setStandaloneMode: (standaloneMode) => set({ standaloneMode }),
  setArchive: (archive) =>
    set({
      archive,
      currentDir: "",
      searchQuery: "",
      selectedPaths: new Set(),
    }),
  setLoading: (loading) => set({ loading }),
  setEncodingOverride: (encodingOverride) => set({ encodingOverride }),
  setCurrentDir: (currentDir) =>
    set({ currentDir, selectedPaths: new Set() }),
  setSearchQuery: (searchQuery) => set({ searchQuery }),
  toggleSelect: (path) =>
    set((state) => {
      const next = new Set(state.selectedPaths);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return { selectedPaths: next };
    }),
  clearSelection: () => set({ selectedPaths: new Set() }),
  selectAll: (paths) => set({ selectedPaths: new Set(paths) }),

  startTask: (activeTask) => set({ activeTask }),
  updateTaskProgress: (progress) =>
    set((state) => {
      if (!state.activeTask || state.activeTask.taskId !== progress.taskId) {
        return state;
      }
      // 已终结状态不再被滞后事件覆盖
      if (
        state.activeTask.status === "completed" ||
        state.activeTask.status === "cancelled" ||
        state.activeTask.status === "error"
      ) {
        return state;
      }
      return {
        activeTask: {
          ...state.activeTask,
          status: progress.paused ? "paused" : "running",
          progress,
        },
      };
    }),
  patchActiveTask: (patch) =>
    set((state) => {
      if (!state.activeTask) return state;
      return {
        activeTask: {
          ...state.activeTask,
          ...patch,
        },
      };
    }),
  clearActiveTask: () => set({ activeTask: null }),

  showToast: (toast) =>
    set({ toast: { ...toast, id: `${Date.now()}-${Math.random()}` } }),
  clearToast: () => set({ toast: null }),
  openCompressModal: (inputs = []) =>
    set({ compressModalOpen: true, compressInputs: inputs }),
  closeCompressModal: () => set({ compressModalOpen: false }),
  setCompressInputs: (compressInputs) => set({ compressInputs }),
  resetArchiveView: () =>
    set({
      archive: null,
      currentDir: "",
      searchQuery: "",
      selectedPaths: new Set(),
      encodingOverride: "auto",
    }),
}));
