use std::{
    thread,
    sync::{Arc, Mutex},
    sync::mpsc,
};

type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
    
}

impl ThreadPool {
    pub fn new(n: usize) -> ThreadPool {
        let mut workers = Vec::with_capacity(n);
        let (sender, receiver) = mpsc::channel();
        let rc = Arc::new(Mutex::new(receiver));
        for _ in 0..n {
            let rc = Arc::clone(&rc);
            let worker = Worker::new(rc); 
            workers.push(worker);
        }
        
        ThreadPool { 
            workers, 
            sender: Some(sender),
        }
    }

    pub fn execute<F: FnOnce() + Send + 'static>(&self, f: F) {
        let job = Box::new(f);
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }

    }
}

impl Worker {
    fn new(rc: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let message = rc.lock().unwrap().recv();

                match message {
                    Ok(job) => job(),
                    Err(_) => break,
                };
                
            }
        });
        Worker { 
            thread: Some(thread)
        }
    }
}
