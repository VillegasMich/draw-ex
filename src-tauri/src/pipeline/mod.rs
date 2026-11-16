//! Owned by D2 (docs/ROADMAP.md): the frame pipeline that runs recognition on submitted frames off the IPC thread.

use std::io;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use draw_core::{Detection, FakeRecognizer, FrameRef, Recognize, RecognizeError};
use tauri::async_runtime::{Sender, channel};

/// Why the pipeline returned no detections for a frame.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    /// The pipeline was shut down before the frame was recognized.
    #[error("the recognition pipeline has stopped")]
    Stopped,
    /// The recognizer failed on this frame.
    #[error("recognition failed: {0}")]
    Recognize(#[from] RecognizeError),
}

/// How many frames may wait for the worker. With at most one frame in flight per
/// camera, the queue never holds more than the number of cameras.
const QUEUE_CAPACITY: usize = 4;

/// The name of the worker thread, as it shows in debuggers and crash reports.
const WORKER_NAME: &str = "draw-ex-recognition";

type Reply = Result<Vec<Detection>, RecognizeError>;

/// One frame waiting for the worker. It owns a copy of the pixels, which the worker
/// drops as soon as this frame is recognized.
struct Job {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    reply: Sender<Reply>,
}

/// The one recognition worker thread, shared by every camera.
///
/// The thread owns the [`Recognize`] implementation and recognizes frames one at a time,
/// in the order they arrive. Callers await the result without blocking their async
/// runtime. Frames wait in a bounded queue, so a slow recognizer makes callers wait
/// rather than letting frames pile up.
#[derive(Debug)]
pub struct Pipeline {
    /// `None` once shut down. Dropping the sender is what tells the worker to stop.
    jobs: Mutex<Option<Sender<Job>>>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl Pipeline {
    /// Starts the worker thread, which owns `recognizer` until [`Pipeline::shutdown`].
    ///
    /// # Errors
    ///
    /// Returns the OS error if the thread cannot be started.
    pub fn spawn(mut recognizer: Box<dyn Recognize>) -> io::Result<Self> {
        let (jobs, mut queue) = channel::<Job>(QUEUE_CAPACITY);
        let worker = thread::Builder::new()
            .name(WORKER_NAME.into())
            .spawn(move || {
                while let Some(job) = queue.blocking_recv() {
                    let frame = FrameRef::new(job.width, job.height, &job.rgba)
                        .expect("a job's pixels are a copy of a valid FrameRef");
                    let reply = recognizer.recognize(frame);
                    // Free the pixels before waiting for the next frame.
                    drop(job.rgba);
                    // The caller may have stopped waiting (e.g. its webview reloaded);
                    // nobody else wants the result then.
                    let _ = job.reply.try_send(reply);
                }
            })?;
        Ok(Self {
            jobs: Mutex::new(Some(jobs)),
            worker: Mutex::new(Some(worker)),
        })
    }

    /// Recognizes `frame` on the worker thread, after the frames submitted before it,
    /// and returns its detections in `frame`'s pixels. The pixels are copied, so the
    /// caller's buffer is free as soon as this is called.
    ///
    /// # Errors
    ///
    /// [`PipelineError::Stopped`] after shutdown, or [`PipelineError::Recognize`] if the
    /// recognizer fails on this frame.
    pub async fn recognize(&self, frame: FrameRef<'_>) -> Result<Vec<Detection>, PipelineError> {
        let (reply, mut result) = channel(1);
        let job = Job {
            width: frame.width(),
            height: frame.height(),
            rgba: frame.rgba().to_vec(),
            reply,
        };
        // Clone the sender out of the lock: the guard must not live across `.await`.
        let jobs = self.jobs().ok_or(PipelineError::Stopped)?;
        jobs.send(job).await.map_err(|_| PipelineError::Stopped)?;
        // Not holding a sender while waiting lets `shutdown` close the queue.
        drop(jobs);
        Ok(result.recv().await.ok_or(PipelineError::Stopped)??)
    }

    /// Stops the worker and waits for its thread to finish the frame it is on and exit.
    /// Frames submitted afterwards fail with [`PipelineError::Stopped`]. Calling it again
    /// does nothing.
    pub fn shutdown(&self) {
        // Closing the queue ends the worker's loop once it has drained the queued frames.
        drop(lock(&self.jobs).take());
        let worker = lock(&self.worker).take();
        if let Some(worker) = worker
            && worker.join().is_err()
        {
            // Its panic message has already been printed by the panic hook.
            eprintln!("draw-ex: the recognition worker thread panicked");
        }
    }

    /// How many frames are waiting for the worker, not counting the one it is on.
    #[cfg(test)]
    fn queued(&self) -> usize {
        self.jobs()
            .map_or(0, |jobs| jobs.max_capacity() - jobs.capacity())
    }

    fn jobs(&self) -> Option<Sender<Job>> {
        lock(&self.jobs).clone()
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// The debug-build environment variable that slows the stand-in recognizer down, in
/// milliseconds per frame, to exercise backpressure. Release builds ignore it.
pub const FAKE_DELAY_ENV: &str = "DRAW_EX_FAKE_RECOGNIZER_DELAY_MS";

/// A stand-in for real recognition (A3/I1): returns preset fake detections, built for
/// each frame's size, through draw-core's [`FakeRecognizer`] so its delay applies.
#[derive(Debug, Clone)]
pub struct CannedRecognizer {
    detections_for: fn(u32, u32) -> Vec<Detection>,
    delay: Option<Duration>,
}

impl CannedRecognizer {
    /// Answers a `width` × `height` frame with `detections_for(width, height)`, after
    /// waiting `delay` if one is set.
    #[must_use]
    pub fn new(detections_for: fn(u32, u32) -> Vec<Detection>, delay: Option<Duration>) -> Self {
        Self {
            detections_for,
            delay,
        }
    }
}

impl Recognize for CannedRecognizer {
    fn recognize(&mut self, frame: FrameRef<'_>) -> Result<Vec<Detection>, RecognizeError> {
        let detections = (self.detections_for)(frame.width(), frame.height());
        FakeRecognizer::new(detections, self.delay).recognize(frame)
    }
}

/// The stand-in recognizer's delay from [`FAKE_DELAY_ENV`] in debug builds; `None` when
/// it is unset, `0`, not a whole number of milliseconds, or in a release build.
#[must_use]
pub fn fake_delay_from_env() -> Option<Duration> {
    if !cfg!(debug_assertions) {
        return None;
    }
    let value = std::env::var(FAKE_DELAY_ENV).ok()?;
    match value.trim().parse::<u64>() {
        Ok(0) => None,
        Ok(ms) => Some(Duration::from_millis(ms)),
        Err(_) => {
            eprintln!("draw-ex: ignoring {FAKE_DELAY_ENV}={value:?}: not whole milliseconds");
            None
        }
    }
}

/// Locks `mutex`, even if a thread panicked while holding it: every value guarded here
/// is a plain `Option` that a panic cannot leave half-updated.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Waker};
    use std::thread::{self, ThreadId};

    use tauri::async_runtime::block_on;

    use super::*;

    /// What the test recognizer saw: each frame's width, and the thread it ran on.
    type Seen = Arc<Mutex<Vec<(u32, ThreadId)>>>;

    /// Records every frame it is given and returns no detections.
    struct Recorder {
        seen: Seen,
    }

    impl Recognize for Recorder {
        fn recognize(&mut self, frame: FrameRef<'_>) -> Result<Vec<Detection>, RecognizeError> {
            self.seen
                .lock()
                .expect("not poisoned")
                .push((frame.width(), thread::current().id()));
            Ok(Vec::new())
        }
    }

    /// Holds each frame until the test releases it, like a recognizer that takes as
    /// long as the test wants. It reports each frame's width when it starts on it.
    struct Gate {
        started: SyncSender<u32>,
        release: Receiver<()>,
    }

    impl Recognize for Gate {
        fn recognize(&mut self, frame: FrameRef<'_>) -> Result<Vec<Detection>, RecognizeError> {
            self.started.send(frame.width()).expect("the test listens");
            self.release.recv().expect("the test releases every frame");
            Ok(Vec::new())
        }
    }

    /// The test's end of a [`Gate`]: when a frame starts, and when to let it finish.
    struct GateControl {
        started: Receiver<u32>,
        release: SyncSender<()>,
    }

    fn gate() -> (Box<dyn Recognize>, GateControl) {
        let (started_tx, started) = sync_channel(QUEUE_CAPACITY * 2);
        let (release, release_rx) = sync_channel(QUEUE_CAPACITY * 2);
        let gate = Gate {
            started: started_tx,
            release: release_rx,
        };
        (Box::new(gate), GateControl { started, release })
    }

    /// Recognizes nothing, and records when it is dropped: the worker thread owns it,
    /// so it is dropped exactly when that thread exits.
    struct DropFlag(Arc<AtomicBool>);

    impl Recognize for DropFlag {
        fn recognize(&mut self, _frame: FrameRef<'_>) -> Result<Vec<Detection>, RecognizeError> {
            Ok(Vec::new())
        }
    }

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    fn pixels(width: u32) -> Vec<u8> {
        vec![0; width as usize * 4]
    }

    fn frame(rgba: &[u8]) -> FrameRef<'_> {
        FrameRef::new((rgba.len() / 4) as u32, 1, rgba).expect("a width × 1 frame")
    }

    /// Polls `future` once without a runtime, so whatever it does synchronously happens now.
    fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
        future.poll(&mut Context::from_waker(Waker::noop()))
    }

    #[test]
    fn frames_are_recognized_in_arrival_order_on_one_worker_thread() {
        let seen = Seen::default();
        let pipeline = Pipeline::spawn(Box::new(Recorder {
            seen: Arc::clone(&seen),
        }))
        .expect("the worker starts");
        let frames: Vec<Vec<u8>> = (1..=QUEUE_CAPACITY as u32).map(pixels).collect();

        // Submit every frame before awaiting any, then await them last to first: the
        // order of arrival, not of awaiting, decides the order of recognition.
        let mut pending: Vec<_> = frames
            .iter()
            .map(|rgba| Box::pin(pipeline.recognize(frame(rgba))))
            .collect();
        for future in &mut pending {
            let _ = poll_once(future.as_mut());
        }
        block_on(async {
            for future in pending.into_iter().rev() {
                future.await.expect("recognized");
            }
        });

        let seen = seen.lock().expect("not poisoned");
        let widths: Vec<u32> = seen.iter().map(|(width, _)| *width).collect();
        assert_eq!(widths, [1, 2, 3, 4]);
        let worker = seen[0].1;
        assert_ne!(worker, thread::current().id());
        assert!(seen.iter().all(|(_, thread)| *thread == worker), "{seen:?}");
    }

    #[test]
    fn a_slow_recognizer_does_not_block_the_caller() {
        let (recognizer, control) = gate();
        let pipeline = Pipeline::spawn(recognizer).expect("the worker starts");
        let rgba = pixels(7);
        let mut pending = Box::pin(pipeline.recognize(frame(&rgba)));

        // The worker is now busy on the frame and stays busy until released; polling the
        // caller's future still returns at once instead of waiting for it.
        assert!(poll_once(pending.as_mut()).is_pending());
        assert_eq!(control.started.recv(), Ok(7));
        assert!(poll_once(pending.as_mut()).is_pending());

        control.release.send(()).expect("the worker waits");
        assert_eq!(block_on(pending).expect("recognized"), []);
    }

    #[test]
    fn the_queue_never_holds_more_than_its_capacity() {
        let (recognizer, control) = gate();
        let pipeline = Pipeline::spawn(recognizer).expect("the worker starts");
        let frames: Vec<Vec<u8>> = (1..=QUEUE_CAPACITY as u32 + 4).map(pixels).collect();
        let mut pending: Vec<_> = frames
            .iter()
            .map(|rgba| Box::pin(pipeline.recognize(frame(rgba))))
            .collect();

        // The worker takes the first frame and holds it; every other frame tries to queue.
        let _ = poll_once(pending[0].as_mut());
        assert_eq!(control.started.recv(), Ok(1));
        for future in &mut pending[1..] {
            assert!(poll_once(future.as_mut()).is_pending());
        }
        assert_eq!(pipeline.queued(), QUEUE_CAPACITY);

        // Every frame is still recognized, in order, once the worker gets to it.
        for _ in &frames {
            control.release.send(()).expect("the worker waits");
        }
        block_on(async {
            for future in pending {
                future.await.expect("recognized");
            }
        });
        let started: Vec<u32> = control.started.try_iter().collect();
        assert_eq!(started, (2..=QUEUE_CAPACITY as u32 + 4).collect::<Vec<_>>());
        assert_eq!(pipeline.queued(), 0);
    }

    #[test]
    fn shutdown_joins_the_worker_thread_and_refuses_later_frames() {
        let dropped = Arc::new(AtomicBool::new(false));
        let pipeline =
            Pipeline::spawn(Box::new(DropFlag(Arc::clone(&dropped)))).expect("the worker starts");
        let rgba = pixels(3);
        block_on(pipeline.recognize(frame(&rgba))).expect("recognized before shutdown");

        pipeline.shutdown();

        assert!(
            dropped.load(Ordering::SeqCst),
            "the worker thread has exited"
        );
        assert!(matches!(
            block_on(pipeline.recognize(frame(&rgba))),
            Err(PipelineError::Stopped)
        ));
        pipeline.shutdown();
    }
}
