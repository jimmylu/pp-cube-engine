//! Dedicated compilation threads; admission never blocks the calling thread.
use crate::{CubeService, ServiceError};
use pp_cube_compile::{CompileContext, Compiled};
use pp_cube_model::Query;
use std::{
    fmt,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{mpsc, Arc, Mutex},
    thread,
};
use tokio::sync::oneshot;

pub(crate) type Job = Box<dyn FnOnce() + Send + 'static>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolError {
    InvalidCapacity,
    Busy,
    Unavailable,
    Compile(ServiceError),
}
impl fmt::Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCapacity => f.write_str("worker and queue capacities must be positive"),
            Self::Busy => f.write_str("compilation queue is full"),
            Self::Unavailable => f.write_str("compilation worker is unavailable"),
            Self::Compile(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for PoolError {}

/// Awaitable result; dropping this handle does not cancel accepted work.
pub struct CompileTask(oneshot::Receiver<Result<Compiled, ServiceError>>);
impl CompileTask {
    pub async fn wait(self) -> Result<Compiled, PoolError> {
        self.0
            .await
            .map_err(|_| PoolError::Unavailable)?
            .map_err(PoolError::Compile)
    }
}

/// Fixed workers and bounded waiting jobs. Register versions before sharing the service.
/// Drop closes admission and detaches draining workers without blocking.
pub struct CompilePool {
    service: Arc<CubeService>,
    sender: Option<mpsc::SyncSender<Job>>,
    workers: Vec<thread::JoinHandle<()>>,
}
impl CompilePool {
    pub fn new(service: Arc<CubeService>, workers: usize, queue: usize) -> Result<Self, PoolError> {
        if workers == 0 || queue == 0 {
            return Err(PoolError::InvalidCapacity);
        }
        let (sender, receiver) = mpsc::sync_channel::<Job>(queue);
        let receiver = Arc::new(Mutex::new(receiver));
        let mut pool = Self {
            service,
            sender: Some(sender),
            workers: Vec::new(),
        };
        for index in 0..workers {
            let receiver = Arc::clone(&receiver);
            let worker = thread::Builder::new()
                .name(format!("cube-compile-{index}"))
                .spawn(move || loop {
                    // Never hold this mutex while running a job.
                    let job = receiver.lock().unwrap_or_else(|e| e.into_inner()).recv();
                    match job {
                        Ok(job) => {
                            let _ = catch_unwind(AssertUnwindSafe(job));
                        }
                        Err(_) => break,
                    }
                })
                .map_err(|_| PoolError::Unavailable)?;
            pool.workers.push(worker);
        }
        Ok(pool)
    }

    /// Returns Busy immediately when the waiting queue is full. All planning is on workers.
    pub fn submit(
        &self,
        version: String,
        query: Query,
        context: CompileContext,
    ) -> Result<CompileTask, PoolError> {
        let service = Arc::clone(&self.service);
        let (sender, receiver) = oneshot::channel();
        self.enqueue(Box::new(move || {
            let _ = sender.send(service.compile(&version, &query, &context));
        }))?;
        Ok(CompileTask(receiver))
    }

    pub(crate) fn enqueue(&self, job: Job) -> Result<(), PoolError> {
        self.sender
            .as_ref()
            .ok_or(PoolError::Unavailable)?
            .try_send(job)
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => PoolError::Busy,
                mpsc::TrySendError::Disconnected(_) => PoolError::Unavailable,
            })
    }

    /// Blocking: call outside async runtime threads. Drains accepted work and joins workers.
    /// Cannot forcibly terminate a hung compilation.
    pub fn shutdown(mut self) {
        self.sender.take();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashSet, time::Duration};
    fn make_pool(workers: usize, queue: usize) -> CompilePool {
        CompilePool::new(Arc::new(CubeService::default()), workers, queue).unwrap()
    }
    fn received<T>(receiver: &mpsc::Receiver<T>) -> T {
        receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("worker progress")
    }

    // scenario: B0-API-006
    #[test]
    fn fixed_workers_saturate_without_blocking_and_recover_from_dropped_and_panicking_jobs() {
        let pool = make_pool(2, 1);
        let (started, ready) = mpsc::channel();
        let mut releases = Vec::new();
        let caller = thread::current().id();
        let mut active_ids = HashSet::new();
        for _ in 0..2 {
            let (release, gate) = mpsc::channel();
            releases.push(release);
            let started = started.clone();
            pool.enqueue(Box::new(move || {
                started.send(thread::current().id()).unwrap();
                received(&gate);
            }))
            .unwrap();
            // Ensure the job is running before submitting the next one.
            let id = received(&ready);
            assert_ne!(id, caller);
            active_ids.insert(id);
        }
        assert_eq!(active_ids.len(), 2);
        // Both workers are blocked, so exactly one queued job is accepted.
        let (result, dropped) = oneshot::channel::<()>();
        drop(dropped);
        pool.enqueue(Box::new(move || {
            let _ = result.send(());
            panic!("test job");
        }))
        .unwrap();
        assert_eq!(pool.enqueue(Box::new(|| {})), Err(PoolError::Busy));
        for release in releases {
            release.send(()).unwrap();
        }
        pool.shutdown();
        assert!(matches!(
            CompilePool::new(Arc::new(CubeService::default()), 0, 1),
            Err(PoolError::InvalidCapacity)
        ));
        assert!(matches!(
            CompilePool::new(Arc::new(CubeService::default()), 1, 0),
            Err(PoolError::InvalidCapacity)
        ));

        // A panicked task reports Unavailable and does not kill its worker.
        let pool = make_pool(1, 2);
        let (send, receive) = oneshot::channel();
        pool.enqueue(Box::new(move || {
            let _send = send;
            panic!("task unwind");
        }))
        .unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        assert_eq!(
            runtime.block_on(CompileTask(receive).wait()),
            Err(PoolError::Unavailable)
        );
        pool.shutdown();

        // A single worker must survive unwind and keep its identity.
        let pool = make_pool(1, 4);
        let (send, recv) = mpsc::channel();
        for panic_job in [false, true, false] {
            let send = send.clone();
            pool.enqueue(Box::new(move || {
                send.send(thread::current().id()).unwrap();
                assert!(!panic_job, "test unwind");
            }))
            .unwrap();
        }
        let ids: HashSet<_> = (0..3).map(|_| received(&recv)).collect();
        assert_eq!(ids.len(), 1);
        pool.shutdown();
    }

    // scenario: B0-API-007
    #[test]
    fn drop_does_not_wait_and_accepted_work_drains() {
        let pool = make_pool(1, 2);
        let (release, gate) = mpsc::channel();
        let (started, ready) = mpsc::channel();
        let (done, complete) = mpsc::channel();
        pool.enqueue(Box::new(move || {
            started.send(()).unwrap();
            received(&gate);
        }))
        .unwrap();
        received(&ready);
        pool.enqueue(Box::new(move || {
            done.send(()).unwrap();
        }))
        .unwrap();
        let (dropped, signal) = mpsc::channel();
        let closer = thread::spawn(move || {
            drop(pool);
            dropped.send(()).unwrap();
        });
        // Must return before the held compilation is released.
        received(&signal);
        release.send(()).unwrap();
        received(&complete);
        closer.join().unwrap();
    }

    // scenario: B0-API-007
    #[test]
    fn shutdown_joins_workers_after_all_accepted_results() {
        let pool = make_pool(2, 4);
        let (send, recv) = mpsc::channel();
        for _ in 0..4 {
            let send = send.clone();
            pool.enqueue(Box::new(move || {
                send.send(()).unwrap();
            }))
            .unwrap();
        }
        pool.shutdown();
        for _ in 0..4 {
            received(&recv);
        }
    }
}
