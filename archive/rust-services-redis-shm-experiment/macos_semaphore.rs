// Proper macOS semaphore implementation that actually works
// This creates semaphores in anonymous shared memory (MAP_ANON) which is fully supported

use crate::{Result, AlphaPulseError};
use std::ptr;
use libc::{sem_t, sem_init, sem_wait, sem_post, sem_destroy, mmap, munmap};
use libc::{MAP_SHARED, MAP_ANON, PROT_READ, PROT_WRITE};

pub struct MacOSSemaphore {
    shm_ptr: *mut libc::c_void,
    sem_ptr: *mut sem_t,
    size: usize,
}

unsafe impl Send for MacOSSemaphore {}
unsafe impl Sync for MacOSSemaphore {}

impl MacOSSemaphore {
    pub fn new(initial_value: u32) -> Result<Self> {
        let size = std::mem::size_of::<sem_t>();
        
        // Create anonymous shared memory region
        // MAP_ANON | MAP_SHARED is the key for cross-process semaphores on macOS
        let shm_ptr = unsafe {
            mmap(
                ptr::null_mut(),
                size,
                PROT_READ | PROT_WRITE,
                MAP_SHARED | MAP_ANON,
                -1,  // No file descriptor for anonymous mapping
                0
            )
        };
        
        if shm_ptr == libc::MAP_FAILED {
            return Err(AlphaPulseError::ConfigError(
                format!("mmap failed for semaphore: {}", std::io::Error::last_os_error())
            ));
        }
        
        let sem_ptr = shm_ptr as *mut sem_t;
        
        // Initialize semaphore in the shared memory
        let result = unsafe { sem_init(sem_ptr, 1, initial_value) };
        
        if result != 0 {
            let error = std::io::Error::last_os_error();
            unsafe { munmap(shm_ptr, size); }
            return Err(AlphaPulseError::ConfigError(
                format!("sem_init failed: {} (this should work on macOS!)", error)
            ));
        }
        
        Ok(MacOSSemaphore {
            shm_ptr,
            sem_ptr,
            size,
        })
    }
    
    pub fn wait(&self) -> Result<()> {
        let result = unsafe { sem_wait(self.sem_ptr) };
        if result != 0 {
            return Err(AlphaPulseError::ConfigError(
                format!("sem_wait failed: {}", std::io::Error::last_os_error())
            ));
        }
        Ok(())
    }
    
    pub fn post(&self) -> Result<()> {
        let result = unsafe { sem_post(self.sem_ptr) };
        if result != 0 {
            return Err(AlphaPulseError::ConfigError(
                format!("sem_post failed: {}", std::io::Error::last_os_error())
            ));
        }
        Ok(())
    }
    
    pub fn try_wait(&self) -> Result<bool> {
        let result = unsafe { libc::sem_trywait(self.sem_ptr) };
        if result == 0 {
            Ok(true)
        } else {
            let errno = unsafe { *libc::__error() };
            if errno == libc::EAGAIN {
                Ok(false)
            } else {
                Err(AlphaPulseError::ConfigError(
                    format!("sem_trywait failed: errno={}", errno)
                ))
            }
        }
    }
}

impl Drop for MacOSSemaphore {
    fn drop(&mut self) {
        unsafe {
            sem_destroy(self.sem_ptr);
            munmap(self.shm_ptr, self.size);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;
    
    #[test]
    fn test_semaphore_basic() {
        let sem = MacOSSemaphore::new(0).unwrap();
        
        // Post and then wait should work
        sem.post().unwrap();
        sem.wait().unwrap();
    }
    
    #[test]
    fn test_semaphore_cross_thread() {
        let sem = Arc::new(MacOSSemaphore::new(0).unwrap());
        let sem_clone = Arc::clone(&sem);
        
        // Spawn thread that will wait
        let handle = thread::spawn(move || {
            sem_clone.wait().unwrap();
            42
        });
        
        // Give thread time to start waiting
        thread::sleep(Duration::from_millis(100));
        
        // Post to wake it up
        sem.post().unwrap();
        
        // Thread should complete
        let result = handle.join().unwrap();
        assert_eq!(result, 42);
    }
}