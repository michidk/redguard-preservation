use std::cell::RefCell;
use std::fmt::Display;
use std::mem::ManuallyDrop;
use std::ptr;

#[repr(C)]
pub struct ByteBuffer {
    pub ptr: *mut u8,
    pub length: i32,
    pub capacity: i32,
}

impl ByteBuffer {
    pub fn from_vec(bytes: Vec<u8>) -> Self {
        let mut bytes = ManuallyDrop::new(bytes);
        let length = i32::try_from(bytes.len()).expect("buffer length exceeds i32::MAX");
        let capacity = i32::try_from(bytes.capacity()).expect("buffer capacity exceeds i32::MAX");
        Self {
            ptr: bytes.as_mut_ptr(),
            length,
            capacity,
        }
    }

    pub fn destroy(self) {
        if self.ptr.is_null() {
            return;
        }

        if self.length < 0 || self.capacity < 0 || self.length > self.capacity {
            return;
        }

        unsafe {
            let _ = Vec::from_raw_parts(self.ptr, self.length as usize, self.capacity as usize);
        }
    }

    pub fn null() -> Self {
        Self {
            ptr: ptr::null_mut(),
            length: 0,
            capacity: 0,
        }
    }
}

thread_local! {
    static LAST_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub fn set_last_error(err: impl Display) {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = Some(err.to_string());
    });
}

pub fn clear_last_error() {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

pub fn last_error_message() -> Option<String> {
    LAST_ERROR.with(|cell| cell.borrow().clone())
}

pub(crate) fn into_ffi_result(result: crate::Result<Vec<u8>>) -> *mut ByteBuffer {
    match result {
        Ok(bytes) => {
            clear_last_error();
            Box::into_raw(Box::new(ByteBuffer::from_vec(bytes)))
        }
        Err(err) => {
            set_last_error(err);
            ptr::null_mut()
        }
    }
}
