//! This module is the process scheduler. It decides the order and
//! other aspects of running user space processes

use alloc::collections::VecDeque;

use crate::process::*;


/// This represents a round robin queue of processes that can be
/// executed now or can be executed at some point in the future.
///
/// This is a struct instead of global to allow for hart affinity via
/// seperate queues, and to ensure that locking and synchronization
/// overhead is only incurred when it is required, by making it
/// optional above this struct.
pub struct ProcessQueue {
    proc_queue: VecDeque<Process>,
}

impl ProcessQueue {
    pub fn new() -> Self {
        Self {
            proc_queue: VecDeque::new()
        }
    }

    /// This is the acquiring half of the scheduler. This function
    /// internally enforces fairness and efficiency and everything else
    pub fn get_ready_process(&mut self) -> Process {
        // iterate while the queue is non-empty
        while let Some(head) = self.proc_queue.pop_front() {
            match head.state {
                // found something we can run
                ProcessState::Ready | ProcessState::Unstarted => {
                    return head
                },

                // found something we might be able to run, check. If
                // ready, run. If not, insert at the end of the queue
                ProcessState::Wait => {
                    todo!("Setup blocking infrastructure")
                },
                ProcessState::Sleep => {
                    todo!("Compare with proc.sleep_time")
                },

                // found something that probably shouldn't be in the queue
                ProcessState::Uninitialized => {
                    panic!("Uninitialized process in scheduling queue!");
                },
                ProcessState::Running => {
                    panic!("Running process in scheduling queue!")
                },
                ProcessState::Dead => {
                    // TODO we need to decide what dead means, and how
                    // dead processes are desposed of
                    todo!("Dead process in scheduling queue!")
                },

            }
        }
        // The queue must be empty, there is nothing to run

        // TODO This need to communicate with other harts to make sure
        // it's not just that the other harts are running / own
        // everything currently
        panic!("Scheduling queue is empty! The root process died?");
    }

    /// This is for returning a process that has just stopped running but
    /// is not completed to the scheduling queue. Either it yielded or
    /// blocked or slept or something. The caller has responsibility to
    /// alter the Process structure to match its state, and then moves it
    /// here to be restarted/started later
    pub fn insert(&mut self, proc: Process) {
        match proc.state {
            ProcessState::Ready | ProcessState::Unstarted => {},
            _ => {
                panic!("Unsuitable process state inserted into scheduling queue! {:?}", proc.state);
            }
        }
        self.proc_queue.push_back(proc);
    }
}

