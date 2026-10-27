//! PS5 Threading and Priority Management
//!
//! Provides thread priority and affinity management for real-time audio/video.
//! On Linux, uses pthread APIs. On PS5/FreeBSD, provides stubs.

#[cfg(feature = "linux-input")]
use num_cpus;

use std::thread;
use std::sync::Arc;

use anyhow::{Context, Result};
use libc::{pthread_t, sched_param, SCHED_FIFO, SCHED_RR, SCHED_OTHER};
use parking_lot::Mutex;
use tracing::{debug, info, warn};

use crate::{DiscordError, Result as PlatformResult};

/// Thread priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadPriority {
    /// Normal priority
    Normal = 0,
    /// High priority for UI
    High = 10,
    /// Real-time priority for audio
    RealTime = 50,
    /// Critical real-time for audio callback
    Critical = 90,
}

/// Thread affinity mask
#[derive(Debug, Clone, Copy)]
pub struct AffinityMask(pub u64);

impl AffinityMask {
    pub fn single_cpu(cpu: usize) -> Self {
        Self(1 << cpu)
    }

    pub fn all_cpus(num_cpus: usize) -> Self {
        Self((1u64 << num_cpus) - 1)
    }
}

/// Set current thread priority
pub fn set_thread_priority(priority: ThreadPriority) -> PlatformResult<()> {
    let policy = match priority {
        ThreadPriority::Normal => SCHED_OTHER,
        ThreadPriority::High => SCHED_RR,
        ThreadPriority::RealTime => SCHED_FIFO,
        ThreadPriority::Critical => SCHED_FIFO,
    };

    let param = sched_param {
        sched_priority: priority as i32,
    };

    // On FreeBSD/PS5, we need to use pthread_setschedparam
    // This is a simplified version - real implementation would use proper syscalls
    unsafe {
        let thread = libc::pthread_self();
        let result = libc::pthread_setschedparam(thread, policy, &param);
        if result != 0 {
            warn!("Failed to set thread priority: {}", std::io::Error::last_os_error());
            // Don't fail - priority is best effort
        }
    }

    Ok(())
}

/// Set thread affinity
pub fn set_thread_affinity(mask: AffinityMask) -> PlatformResult<()> {
    // On PS5/FreeBSD, use cpuset_setaffinity
    // Simplified for now
    Ok(())
}

/// Spawn a thread with specific priority and affinity
pub fn spawn_priority_thread<F>(name: &str, priority: ThreadPriority, affinity: Option<AffinityMask>, f: F) -> PlatformResult<thread::JoinHandle<()>>
where
    F: FnOnce() + Send + 'static,
{
    let name = name.to_string();

    let handle = thread::Builder::new()
        .name(name.clone())
        .spawn(move || {
            if let Err(e) = set_thread_priority(priority) {
                warn!("Failed to set priority for thread '{}': {}", name, e);
            }

            if let Some(affinity) = affinity {
                if let Err(e) = set_thread_affinity(affinity) {
                    warn!("Failed to set affinity for thread '{}': {}", name, e);
                }
            }

            info!("Thread '{}' started with priority {:?}", name, priority);
            f();
            info!("Thread '{}' finished", name);
        })
        .context("Failed to spawn thread")?;

    Ok(handle)
}

/// Thread pool for background tasks
pub struct BackgroundThreadPool {
    handles: Mutex<Vec<thread::JoinHandle<()>>>,
}

impl BackgroundThreadPool {
    pub fn new() -> Self {
        Self {
            handles: Mutex::new(Vec::new()),
        }
    }

    pub fn spawn<F>(&self, name: &str, f: F) -> PlatformResult<()>
    where
        F: FnOnce() + Send + 'static,
    {
        let handle = thread::Builder::new()
            .name(name.to_string())
            .spawn(f)
            .context("Failed to spawn background thread")?;

        self.handles.lock().push(handle);
        Ok(())
    }

    pub fn join_all(self) {
        for handle in self.handles.into_inner() {
            let _ = handle.join();
        }
    }
}

impl Default for BackgroundThreadPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Scoped thread for RAII joining
pub struct ScopedThread<'a> {
    handle: Option<thread::JoinHandle<()>>,
    name: String,
}

impl<'a> ScopedThread<'a> {
    pub fn spawn<F>(name: &str, f: F) -> Self
    where
        F: FnOnce() + Send + 'a,
    {
        let handle = thread::Builder::new()
            .name(name.to_string())
            .spawn(f)
            .expect("Failed to spawn scoped thread");

        Self {
            handle: Some(handle),
            name: name.to_string(),
        }
    }
}

impl<'a> Drop for ScopedThread<'a> {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Initialize threading subsystem
pub fn init_threading() -> PlatformResult<()> {
    info!("Initializing threading subsystem");

    // Set main thread priority
    set_thread_priority(ThreadPriority::High)?;

    // Log CPU count
    #[cfg(feature = "linux-input")]
    let num_cpus = num_cpus::get();
    #[cfg(not(feature = "linux-input"))]
    let num_cpus = 8; // PS5 has 8 cores

    info!("Detected {} CPU cores", num_cpus);

    Ok(())
}