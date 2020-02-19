use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

enum Message {
    Job(Box<dyn FnOnce() + Send + 'static>),
    Shutdown,
}

pub struct Pool {
    workers: Vec<Worker>,
    chan: Sender<Message>,
    size: usize,
}

struct Worker {
    thread: thread::JoinHandle<()>,
}

impl Pool {
    /// Start a new thread pool.
    pub fn new(size: usize) -> Pool {
        let (send, recv) = channel();
        let recv = Arc::new(Mutex::new(recv));
        let mut pool = Pool {
            workers: vec![],
            chan: send,
            size,
        };

        pool.workers.reserve(size);
        for _ in 0..size {
            pool.workers.push(Worker::spawn(recv.clone()));
        }

        pool
    }

    /// Schedule a new job to be scheduled onto the
    /// thread pool.
    pub fn schedule<F: FnOnce() + Send + 'static>(&self, job: F) {
        if self.size == 0 {
            panic!("attempting to use pool that was shutdown");
        }
        self.chan.send(Message::Job(Box::new(job))).unwrap();
    }

    /// Terminate the pool after completing
    /// all outstanding jobs.
    pub fn shutdown(&mut self) {
        for _ in 0..self.size {
            self.chan.send(Message::Shutdown).unwrap();
        }
        while self.workers.len() > 0 {
            self.workers.pop().unwrap().join();
        }
        self.size = 0;
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        self.shutdown()
    }
}

impl Worker {
    fn spawn(recv: Arc<Mutex<Receiver<Message>>>) -> Worker {
        Worker {
            thread: thread::spawn(move || {
                loop {
                    // assign to var to drop lock after assignment
                    let msg = recv.lock().unwrap().recv().unwrap();
                    match msg {
                        Message::Job(job) => job(),
                        Message::Shutdown => return,
                    }
                }
            }),
        }
    }

    fn join(self) {
        self.thread.join().unwrap();
    }
}
