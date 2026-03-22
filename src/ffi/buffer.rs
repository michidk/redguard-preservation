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
    #[must_use]
    pub fn from_vec(bytes: Vec<u8>) -> Self {
        let Ok(length) = i32::try_from(bytes.len()) else {
            set_last_error("buffer length exceeds i32::MAX");
            return Self::null();
        };
        let Ok(capacity) = i32::try_from(bytes.capacity()) else {
            set_last_error("buffer capacity exceeds i32::MAX");
            return Self::null();
        };

        let mut bytes = ManuallyDrop::new(bytes);
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

        let Ok(length) = usize::try_from(self.length) else {
            return;
        };
        let Ok(capacity) = usize::try_from(self.capacity) else {
            return;
        };

        unsafe {
            let _ = Vec::from_raw_parts(self.ptr, length, capacity);
        }
    }

    #[must_use]
    pub const fn null() -> Self {
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

#[must_use]
pub fn last_error_message() -> Option<String> {
    LAST_ERROR.with(|cell| cell.borrow().clone())
}

pub fn into_ffi_result(result: crate::Result<Vec<u8>>) -> *mut ByteBuffer {
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
