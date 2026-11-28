//! 任务控制（暂停 / 恢复 / 取消）与字节级进度追踪

use crate::error::{QPackError, QResult};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

struct ControlInner {
    cancelled: AtomicBool,
    paused: Mutex<bool>,
    cond: Condvar,
}

/// 可在多线程间共享的任务控制器
#[derive(Clone)]
pub struct TaskController {
    inner: Arc<ControlInner>,
}

impl Default for TaskController {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskController {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(ControlInner {
                cancelled: AtomicBool::new(false),
                paused: Mutex::new(false),
                cond: Condvar::new(),
            }),
        }
    }

    /// 不会被外部控制的占位控制器（无界面场景 / 测试）
    pub fn noop() -> Self {
        Self::new()
    }

    pub fn pause(&self) {
        *self.inner.paused.lock().unwrap() = true;
    }

    pub fn resume(&self) {
        let mut guard = self.inner.paused.lock().unwrap();
        *guard = false;
        self.inner.cond.notify_all();
    }

    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::SeqCst);
        let _guard = self.inner.paused.lock().unwrap();
        self.inner.cond.notify_all();
    }

    pub fn is_paused(&self) -> bool {
        *self.inner.paused.lock().unwrap()
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    /// 检查任务状态：已取消返回 `Cancelled`；已暂停则阻塞直到恢复或取消
    pub fn check_state(&self) -> QResult<()> {
        if self.is_cancelled() {
            return Err(QPackError::Cancelled);
        }
        let mut guard = self.inner.paused.lock().unwrap();
        while *guard {
            if self.is_cancelled() {
                return Err(QPackError::Cancelled);
            }
            guard = self.inner.cond.wait(guard).unwrap();
        }
        if self.is_cancelled() {
            Err(QPackError::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// 任务注册表：按任务 ID 管理所有进行中的任务控制器
#[derive(Default)]
pub struct TaskRegistry {
    tasks: Mutex<HashMap<String, TaskController>>,
}

impl TaskRegistry {
    pub fn register(&self, task_id: &str) -> TaskController {
        let controller = TaskController::new();
        self.tasks
            .lock()
            .unwrap()
            .insert(task_id.to_string(), controller.clone());
        controller
    }

    pub fn remove(&self, task_id: &str) {
        self.tasks.lock().unwrap().remove(task_id);
    }

    fn get(&self, task_id: &str) -> Option<TaskController> {
        self.tasks.lock().unwrap().get(task_id).cloned()
    }

    pub fn pause(&self, task_id: &str) -> bool {
        self.get(task_id).map(|c| c.pause()).is_some()
    }

    pub fn resume(&self, task_id: &str) -> bool {
        self.get(task_id).map(|c| c.resume()).is_some()
    }

    pub fn cancel(&self, task_id: &str) -> bool {
        self.get(task_id).map(|c| c.cancel()).is_some()
    }
}

/// 进度快照
#[derive(Debug, Clone, Default)]
pub struct ProgressSnapshot {
    pub current_file: String,
    pub processed_items: usize,
    pub total_items: usize,
    pub processed_bytes: u64,
    pub total_bytes: u64,
    pub current_file_bytes: u64,
    pub current_file_total_bytes: u64,
    pub elapsed_ms: u64,
    pub speed_bps: f64,
    pub paused: bool,
    pub done: bool,
}

/// 进度回调函数签名
pub type ProgressCallback<'a> = &'a mut dyn FnMut(&ProgressSnapshot);

const EMIT_INTERVAL: Duration = Duration::from_millis(60);

/// 字节级进度追踪器：节流上报、统计耗时（不含暂停时间）与速度，并负责暂停/取消检查点
pub struct ProgressTracker<'a> {
    control: TaskController,
    callback: ProgressCallback<'a>,
    started: Instant,
    paused_total: Duration,
    last_emit: Option<Instant>,
    state: ProgressSnapshot,
}

impl<'a> ProgressTracker<'a> {
    pub fn new(control: TaskController, callback: ProgressCallback<'a>) -> Self {
        Self {
            control,
            callback,
            started: Instant::now(),
            paused_total: Duration::ZERO,
            last_emit: None,
            state: ProgressSnapshot::default(),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.control.is_cancelled()
    }

    pub fn set_totals(&mut self, items: usize, bytes: u64) {
        self.state.total_items = items;
        self.state.total_bytes = bytes;
        self.emit(true);
    }

    /// 检查点：处理暂停（阻塞等待）与取消
    pub fn checkpoint(&mut self) -> QResult<()> {
        if self.control.is_paused() && !self.control.is_cancelled() {
            let since = Instant::now();
            self.state.paused = true;
            self.emit(true);
            let res = self.control.check_state();
            self.paused_total += since.elapsed();
            self.state.paused = false;
            self.emit(true);
            res
        } else {
            self.control.check_state()
        }
    }

    pub fn begin_file(&mut self, name: &str, size: u64) {
        self.state.current_file = name.to_string();
        self.state.current_file_bytes = 0;
        self.state.current_file_total_bytes = size;
        self.emit(false);
    }

    pub fn add_bytes(&mut self, n: u64) {
        self.state.processed_bytes = self.state.processed_bytes.saturating_add(n);
        self.state.current_file_bytes = self.state.current_file_bytes.saturating_add(n);
        self.emit(false);
    }

    pub fn finish_file(&mut self) {
        self.state.processed_items += 1;
        self.state.current_file_bytes = self
            .state
            .current_file_bytes
            .max(self.state.current_file_total_bytes);
        self.emit(false);
    }

    pub fn finish(&mut self) {
        self.state.processed_items = self.state.total_items.max(self.state.processed_items);
        self.state.processed_bytes = self.state.total_bytes.max(self.state.processed_bytes);
        self.state.current_file_bytes = self.state.current_file_total_bytes;
        self.state.paused = false;
        self.state.done = true;
        self.emit(true);
    }

    fn emit(&mut self, force: bool) {
        let now = Instant::now();
        if !force {
            if let Some(last) = self.last_emit {
                if now.duration_since(last) < EMIT_INTERVAL {
                    return;
                }
            }
        }
        let active = self.started.elapsed().saturating_sub(self.paused_total);
        self.state.elapsed_ms = active.as_millis() as u64;
        let secs = active.as_secs_f64();
        self.state.speed_bps = if secs > 0.05 {
            self.state.processed_bytes as f64 / secs
        } else {
            0.0
        };
        (self.callback)(&self.state);
        self.last_emit = Some(now);
    }
}

/// 带进度与暂停/取消检查的读取器包装
pub struct ProgressReader<'r, 'a, R: Read> {
    inner: R,
    tracker: &'r mut ProgressTracker<'a>,
}

impl<'r, 'a, R: Read> ProgressReader<'r, 'a, R> {
    pub fn new(inner: R, tracker: &'r mut ProgressTracker<'a>) -> Self {
        Self { inner, tracker }
    }
}

impl<R: Read> Read for ProgressReader<'_, '_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if let Err(err) = self.tracker.checkpoint() {
            return Err(io::Error::new(io::ErrorKind::Other, err.to_string()));
        }
        let n = self.inner.read(buf)?;
        self.tracker.add_bytes(n as u64);
        Ok(n)
    }
}

/// 带进度、暂停与取消检查的流拷贝（64KB 缓冲）
pub fn copy_with_progress<R: Read + ?Sized, W: Write + ?Sized>(
    reader: &mut R,
    writer: &mut W,
    tracker: &mut ProgressTracker,
) -> QResult<u64> {
    let mut buf = vec![0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        tracker.checkpoint()?;
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        };
        writer.write_all(&buf[..n])?;
        tracker.add_bytes(n as u64);
        total += n as u64;
    }
    Ok(total)
}
