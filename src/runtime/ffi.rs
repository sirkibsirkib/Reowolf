use crate::common::*;
use crate::runtime::*;

use core::cell::RefCell;
use std::os::raw::{c_char, c_int, c_uchar, c_uint};

struct StoredError {
    filled: bool,
    buf: Vec<c_char>,
}
thread_local! {
    // stores a string. DOES store the null terminator
    static LAST_ERROR: RefCell<StoredError> = RefCell::new(StoredError { filled: false, buf: Vec::with_capacity(128) } );
}

const NULL_TERMINATOR: c_char = b'\0' as c_char;
// Silly HACK: rust uses MAX alignment of 128 bytes for fields (no effect) but causes
// cbindgen tool to make this struct OPAQUE (which is what we want).

// NOT null terminated
fn overwrite_last_error(error_msg: &[u8]) {
    LAST_ERROR.with(|stored_error| {
        let mut stored_error = stored_error.borrow_mut();
        stored_error.filled = true;
        stored_error.buf.clear();
        let error_msg = unsafe { &*(error_msg as *const [u8] as *const [i8]) };
        stored_error.buf.extend_from_slice(error_msg);
        stored_error.buf.push(NULL_TERMINATOR);
    })
}

unsafe fn as_rust_str<R, F: FnOnce(&str) -> R>(s: *const c_char, f: F) -> Option<R> {
    as_rust_bytes(s, |bytes| {
        let s = std::str::from_utf8(bytes).ok()?;
        Some(f(s))
    })
}

unsafe fn as_rust_bytes<R, F: FnOnce(&[u8]) -> R>(s: *const c_char, f: F) -> R {
    let len = c_str_len(s);
    let s = s as *const u8;
    let bytes: &[u8] = std::slice::from_raw_parts(s, len);
    f(bytes)
}

unsafe fn c_str_len(s: *const c_char) -> usize {
    let mut len = 0;
    while *(s.offset(len.try_into().unwrap())) != NULL_TERMINATOR {
        len += 1;
    }
    len
}

unsafe fn try_parse_addr(s: *const c_char) -> Option<SocketAddr> {
    as_rust_str(s, |s| s.parse().ok()).and_then(|x| x)
}

///////////////////////////////////////

/// Returns a pointer into the error buffer for reading as a null-terminated string
/// Returns null if there is no error in the buffer.
/// # Safety
/// TODO
#[no_mangle]
pub unsafe extern "C" fn connector_error_peek() -> *const c_char {
    LAST_ERROR.with(|stored_error| {
        let stored_error = stored_error.borrow();
        if stored_error.filled {
            stored_error.buf.as_ptr()
        } else {
            std::ptr::null()
        }
    })
}

/// Resets the error message buffer.
/// Returns:
/// - 0 if an error was cleared
/// - 1 if there was no error to clear
/// # Safety
/// TODO
#[no_mangle]
pub extern "C" fn connector_error_clear() -> c_int {
    LAST_ERROR.with(|stored_error| {
        let mut stored_error = stored_error.borrow_mut();
        if stored_error.filled {
            stored_error.buf.clear();
            stored_error.filled = false;
            0
        } else {
            1
        }
    })
}

/// Creates and returns Reowolf Connector structure allocated on the heap.
#[no_mangle]
pub extern "C" fn connector_new() -> *mut Connector {
    Box::into_raw(Box::new(Connector::default()))
}

/// Creates and returns Reowolf Connector structure allocated on the heap.
#[no_mangle]
pub extern "C" fn connector_with_controller_id(controller_id: ControllerId) -> *mut Connector {
    Box::into_raw(Box::new(Connector::Unconfigured(Unconfigured { controller_id })))
}

/// Configures the given Reowolf connector with a protocol description in PDL.
/// Returns:
/// # Safety
/// TODO
#[no_mangle]
pub unsafe extern "C" fn connector_configure(
    connector: *mut Connector,
    pdl: *mut c_char,
    main: *mut c_char,
) -> c_int {
    let mut b = Box::from_raw(connector); // unsafe!
    let ret = as_rust_bytes(pdl, |pdl_bytes| {
        as_rust_bytes(main, |main_bytes| match b.configure(pdl_bytes, main_bytes) {
            Ok(()) => 0,
            Err(e) => {
                overwrite_last_error(format!("{:?}", e).as_bytes());
                -1
            }
        })
    });
    Box::into_raw(b); // don't drop!
    ret
}

/// Provides a binding annotation for the port with the given index with "native":
/// (The port is exposed for reading and writing from the application)
/// Returns:
/// # Safety
/// TODO
#[no_mangle]
pub unsafe extern "C" fn connector_bind_native(
    connector: *mut Connector,
    proto_port_index: usize,
) -> c_int {
    // use PortBindErr::*;
    let mut b = Box::from_raw(connector); // unsafe!
    let ret = match b.bind_port(proto_port_index, PortBinding::Native) {
        Ok(()) => 0,
        Err(e) => {
            overwrite_last_error(format!("{:?}", e).as_bytes());
            -1
        }
    };
    Box::into_raw(b); // don't drop!
    ret
}

/// Provides a binding annotation for the port with the given index with "native":
/// (The port is exposed for reading and writing from the application)
/// Returns:
/// # Safety
/// TODO
#[no_mangle]
pub unsafe extern "C" fn connector_bind_passive(
    connector: *mut Connector,
    proto_port_index: c_uint,
    address: *const c_char,
) -> c_int {
    if let Some(addr) = try_parse_addr(address) {
        // use PortBindErr::*;
        let mut b = Box::from_raw(connector); // unsafe!
        let ret =
            match b.bind_port(proto_port_index.try_into().unwrap(), PortBinding::Passive(addr)) {
                Ok(()) => 0,
                Err(e) => {
                    overwrite_last_error(format!("{:?}", e).as_bytes());
                    -1
                }
            };
        Box::into_raw(b); // don't drop!
        ret
    } else {
        overwrite_last_error(b"Failed to parse input as ip address!");
        -1
    }
}

/// Provides a binding annotation for the port with the given index with "active":
/// (The port will conenct to a "passive" port at the given address during connect())
/// Returns:
/// - 0 for success
/// - 1 if the port was already bound and was left unchanged
/// # Safety
/// TODO
#[no_mangle]
pub unsafe extern "C" fn connector_bind_active(
    connector: *mut Connector,
    proto_port_index: c_uint,
    address: *const c_char,
) -> c_int {
    if let Some(addr) = try_parse_addr(address) {
        // use PortBindErr::*;
        let mut b = Box::from_raw(connector); // unsafe!
        let ret = match b.bind_port(proto_port_index.try_into().unwrap(), PortBinding::Active(addr))
        {
            Ok(()) => 0,
            Err(e) => {
                overwrite_last_error(format!("{:?}", e).as_bytes());
                -1
            }
        };
        Box::into_raw(b); // don't drop!
        ret
    } else {
        overwrite_last_error(b"Failed to parse input as ip address!");
        -1
    }
}

/// Provides a binding annotation for the port with the given index with "active":
/// (The port will conenct to a "passive" port at the given address during connect())
/// Returns:
/// - 0 SUCCESS: connected successfully
/// - TODO error codes
/// # Safety
/// TODO
#[no_mangle]
pub unsafe extern "C" fn connector_connect(
    connector: *mut Connector,
    timeout_millis: u64,
) -> c_int {
    let mut b = Box::from_raw(connector); // unsafe!
    let ret = match b.connect(Duration::from_millis(timeout_millis)) {
        Ok(()) => 0,
        Err(e) => {
            overwrite_last_error(format!("{:?}", e).as_bytes());
            -1
        }
    };
    Box::into_raw(b); // don't drop!
    ret
}

/// Destroys the given connector, freeing its underlying resources.
/// # Safety
/// TODO
#[no_mangle]
pub unsafe extern "C" fn connector_destroy(connector: *mut Connector) {
    let c = Box::from_raw(connector); // unsafe!
    drop(c); // for readability
}

/// Prepares to synchronously put a message at the given port, reading it from the given buffer.
/// # Safety
/// TODO
#[no_mangle]
pub unsafe extern "C" fn connector_put(
    connector: *mut Connector,
    proto_port_index: c_uint,
    buf_ptr: *mut c_uchar,
    msg_len: c_uint,
) -> c_int {
    let buf = std::slice::from_raw_parts_mut(buf_ptr, msg_len.try_into().unwrap());
    let vec: Vec<u8> = buf.to_vec(); // unsafe
    let mut b = Box::from_raw(connector); // unsafe!
    let ret = b.put(proto_port_index.try_into().unwrap(), vec.into());
    Box::into_raw(b); // don't drop!
    match ret {
        Ok(()) => 0,
        Err(e) => {
            overwrite_last_error(format!("{:?}", e).as_bytes());
            -1
        }
    }
}

/// Prepares to synchronously put a message at the given port, writing it to the given buffer.
/// - 0 SUCCESS
/// - 1 this port has the wrong direction
/// - 2 this port is already marked to get
/// # Safety
/// TODO
#[no_mangle]
pub unsafe extern "C" fn connector_get(
    connector: *mut Connector,
    proto_port_index: c_uint,
) -> c_int {
    let mut b = Box::from_raw(connector); // unsafe!
    let ret = b.get(proto_port_index.try_into().unwrap());
    Box::into_raw(b); // don't drop!
                      // use PortOperationErr::*;
    match ret {
        Ok(()) => 0,
        Err(e) => {
            overwrite_last_error(format!("{:?}", e).as_bytes());
            -1
        }
    }
}

/// # Safety
/// TODO
#[no_mangle]
pub unsafe extern "C" fn connector_gotten(
    connector: *mut Connector,
    proto_port_index: c_uint,
    buf_ptr_outptr: *mut *const c_uchar,
    len_outptr: *mut c_uint,
) -> c_int {
    let b = Box::from_raw(connector); // unsafe!
    let ret = b.read_gotten(proto_port_index.try_into().unwrap());
    // use ReadGottenErr::*;
    let result = match ret {
        Ok(ptr_slice) => {
            let buf_ptr = ptr_slice.as_ptr();
            let len = ptr_slice.len().try_into().unwrap();
            buf_ptr_outptr.write(buf_ptr);
            len_outptr.write(len);
            0
        }
        Err(e) => {
            overwrite_last_error(format!("{:?}", e).as_bytes());
            -1
        }
    };
    Box::into_raw(b); // don't drop!
    result
}

/// # Safety
/// TODO
#[no_mangle]
pub unsafe extern "C" fn connector_dump_log(connector: *mut Connector) -> c_int {
    let mut b = Box::from_raw(connector); // unsafe!
    let result = match b.get_mut_logger() {
        Some(s) => {
            println!("{}", s);
            0
        }
        None => 1,
    };
    Box::into_raw(b); // don't drop!
    result
}

/// # Safety
/// TODO
#[no_mangle]
pub unsafe extern "C" fn connector_next_batch(connector: *mut Connector) -> c_int {
    let mut b = Box::from_raw(connector); // unsafe!
    let result = match b.next_batch() {
        Ok(batch_index) => batch_index.try_into().unwrap(),
        Err(e) => {
            overwrite_last_error(format!("{:?}", e).as_bytes());
            -1
        }
    };
    Box::into_raw(b); // don't drop!
    result
}

/// # Safety
/// TODO
#[no_mangle]
pub unsafe extern "C" fn connector_sync(connector: *mut Connector, timeout_millis: u64) -> c_int {
    let mut b = Box::from_raw(connector); // unsafe!
    let result = match b.sync(Duration::from_millis(timeout_millis)) {
        Ok(batch_index) => batch_index.try_into().unwrap(),
        Err(SyncErr::Timeout) => -1, // timeout!
        Err(e) => {
            overwrite_last_error(format!("{:?}", e).as_bytes());
            -2
        }
    };
    Box::into_raw(b); // don't drop!
    result
}
