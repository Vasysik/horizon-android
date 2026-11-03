//! SRV through Android's system resolver, including VPN and Private DNS.
//! Resolve API-29 symbols dynamically so API-28 devices can still load the game.
use std::ffi::{CString, c_char, c_int, c_void};

#[repr(C)]
struct PollFd {
    fd: c_int,
    events: i16,
    revents: i16,
}

#[link(name = "dl")]
unsafe extern "C" {
    fn dlopen(name: *const c_char, flags: c_int) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn poll(fds: *mut PollFd, count: usize, timeout: c_int) -> c_int;
}

struct Library(*mut c_void);
impl Drop for Library {
    fn drop(&mut self) {
        // This guard outlives all calls through symbols from the library.
        unsafe {
            dlclose(self.0);
        }
    }
}

/// Complete DNS message, parsed and checked by horizon-protocol's SRV decoder.
pub fn query_srv(name: &str) -> Option<Vec<u8>> {
    type Query = unsafe extern "C" fn(u64, *const c_char, c_int, c_int, u32) -> c_int;
    type Result = unsafe extern "C" fn(c_int, *mut c_int, *mut u8, usize) -> c_int;
    type Cancel = unsafe extern "C" fn(c_int);
    let name = CString::new(name).ok()?;
    // Opening the public NDK library works on both 32-bit and 64-bit Bionic
    // (their RTLD_DEFAULT constants differ), without an API-29 load dependency.
    // Every signature matches android/multinetwork.h in the NDK.
    unsafe {
        let handle = dlopen(c"libandroid.so".as_ptr(), 2);
        if handle.is_null() {
            return None;
        }
        let _library = Library(handle);
        let query = dlsym(handle, c"android_res_nquery".as_ptr());
        let result = dlsym(handle, c"android_res_nresult".as_ptr());
        let cancel = dlsym(handle, c"android_res_cancel".as_ptr());
        if query.is_null() || result.is_null() || cancel.is_null() {
            return None;
        }
        let query: Query = std::mem::transmute(query);
        let result: Result = std::mem::transmute(result);
        let cancel: Cancel = std::mem::transmute(cancel);
        // Network handle 0 selects the calling app's current default network.
        let fd = query(0, name.as_ptr(), 1, 33, 0);
        if fd < 0 {
            return None;
        }
        let mut ready = PollFd {
            fd,
            events: 1,
            revents: 0,
        };
        if poll(&mut ready, 1, 5000) <= 0 || ready.revents & 1 == 0 {
            // Both cancel and result consume/close the descriptor; exactly one
            // must run, including timeout and interrupted-poll paths.
            cancel(fd);
            return None;
        }
        let mut response = vec![0u8; 65535];
        let mut rcode = 0;
        let len = result(fd, &mut rcode, response.as_mut_ptr(), response.len());
        if len < 12 || len as usize > response.len() {
            return None;
        }
        response.truncate(len as usize);
        Some(response)
    }
}
