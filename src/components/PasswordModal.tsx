import { useState } from "react";
import { Lock, X } from "lucide-react";

interface PasswordModalProps {
  open: boolean;
  title?: string;
  onCancel: () => void;
  onConfirm: (password: string) => void;
}

export function PasswordModal({
  open,
  title = "该压缩包已加密，请输入解压密码",
  onCancel,
  onConfirm,
}: PasswordModalProps) {
  const [password, setPassword] = useState("");

  if (!open) return null;

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!password) return;
    onConfirm(password);
    setPassword("");
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 backdrop-blur-xs">
      <form
        onSubmit={submit}
        className="w-full max-w-sm overflow-hidden rounded-2xl border border-zinc-200 bg-white shadow-2xl dark:border-zinc-800 dark:bg-zinc-950"
      >
        <div className="flex items-center justify-between border-b border-zinc-200 px-4 py-3 dark:border-zinc-800">
          <div className="flex items-center gap-2 text-sm font-semibold text-zinc-900 dark:text-zinc-100">
            <Lock className="h-4 w-4 text-amber-500" />
            <span>输入密码</span>
          </div>
          <button
            type="button"
            onClick={onCancel}
            className="cursor-pointer rounded-lg p-1 text-zinc-400 hover:bg-zinc-100 dark:hover:bg-zinc-800"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="p-4">
          <p className="mb-3 text-xs text-zinc-500 dark:text-zinc-400">{title}</p>
          <input
            type="password"
            autoFocus
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder="请输入密码..."
            className="w-full rounded-xl border border-zinc-200 bg-white px-3 py-2 text-sm text-zinc-900 outline-none focus:border-blue-500 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-100"
          />
        </div>

        <div className="flex justify-end gap-2 border-t border-zinc-200 bg-zinc-50/70 px-4 py-2.5 dark:border-zinc-800 dark:bg-zinc-900/50">
          <button
            type="button"
            onClick={onCancel}
            className="cursor-pointer rounded-xl px-3.5 py-1.5 text-xs font-medium text-zinc-600 hover:bg-zinc-200/70 dark:text-zinc-400 dark:hover:bg-zinc-800"
          >
            取消
          </button>
          <button
            type="submit"
            disabled={!password}
            className="cursor-pointer rounded-xl bg-blue-600 px-4 py-1.5 text-xs font-medium text-white hover:bg-blue-700 disabled:opacity-50"
          >
            确认
          </button>
        </div>
      </form>
    </div>
  );
}
