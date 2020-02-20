use parking_lot::Condvar;
use parking_lot::Mutex;

pub struct Lock {
    mutex: Mutex<bool>,
    var: Condvar,
}

impl Lock {
    pub fn new() -> Self {
        Lock {
            mutex: Mutex::new(false),
            var: Condvar::new(),
        }
    }

    pub fn lock(&mut self) {
        let mut locked = self.mutex.lock();
        if *locked {
            self.wait()
        }

        *locked = true;
    }

    fn unlock(&mut self) {
        let mut locked = self.mutex.lock();
        assert!(*locked);
        self.var.notify_one();
    }

    fn wait(&mut self) {
        let mut locked = self.mutex.lock();
        if *locked {
            self.var.wait(&mut locked);
        }
    }
}
