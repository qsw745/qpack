import { create } from "zustand";
import type { ArchiveInfo, EngineStatus, TaskProgress } from "../types/archive";

export interface ToastNotice {
  id: string;
  type: "success" | "error" | "info";
  title: string;
  detail?: string;
  actionPath?: string;
}

interface AppState {
  engineStatus: EngineStatus | null;
  archive: ArchiveInfo | null;
  loading: boolean;
  encodingOverride: string;
  currentDir: string;
  searchQuery: string;
  selectedPaths: Set<string>;
  progress: TaskProgress | null;
  toast: ToastNotice | null;

  // 新建压缩弹窗状态
  compressModalOpen: boolean;
  compressInputs: string[];

  setEngineStatus: (status: EngineStatus) => void;
  setArchive: (info: ArchiveInfo | null) => void;
  setLoading: (loading: boolean) => void;
  setEncodingOverride: (enc: string) => void;
  setCurrentDir: (dir: string) => void;
  setSearchQuery: (q: string) => void;
  toggleSelect: (path: string) => void;
  clearSelection: () => void;
  selectAll: (paths: string[]) => void;
  setProgress: (progress: TaskProgress | null) => void;
  showToast: (toast: Omit<ToastNotice, "id">) => void;
  clearToast: () => void;
  openCompressModal: (inputs?: string[]) => void;
  closeCompressModal: () => void;
  setCompressInputs: (inputs: string[]) => void;
  resetArchiveView: () => void;
}

export const useAppStore = create<AppState>((set) => ({
  engineStatus: null,
  archive: null,
  loading: false,
  encodingOverride: "auto",
  currentDir: "",
  searchQuery: "",
  selectedPaths: new Set(),
  progress: null,
  toast: null,
  compressModalOpen: false,
  compressInputs: [],

  setEngineStatus: (engineStatus) => set({ engineStatus }),
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
  setProgress: (progress) => set({ progress }),
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
