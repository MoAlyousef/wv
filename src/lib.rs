#![allow(non_camel_case_types)]

// Uses code from https://github.com/webview/webview_rust/blob/dev/src/webview.rs

use wv_sys::*;

use std::{
    ffi::{CStr, CString},
    io, fmt,
    string::FromUtf8Error,
    mem,
    os::raw,
    ptr,
    sync::Arc,
};

/// Error types returned by fltk-rs + wrappers of std errors
#[derive(Debug)]
#[non_exhaustive]
pub enum WvError {
    /// i/o error
    IoError(io::Error),
    /// Utf-8 conversion error
    Utf8Error(FromUtf8Error),
    /// Null string conversion error
    NullError(std::ffi::NulError),
    /// Internal fltk error
    Internal(WvErrorKind),
    /// Error using an erroneous env variable
    EnvVarError(std::env::VarError),
    /// Parsing error
    ParseIntError(std::num::ParseIntError),
    /// Unknown error
    Unknown(String),
}

unsafe impl Send for WvError {}
unsafe impl Sync for WvError {}

/// Error kinds enum for `WvError`
#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum WvErrorKind {
    /// Missing dependency
    MissingDependency = -5,
    /// Failed to initialize the multithreading
    OperationCancelled = -4,
    /// Failed to set the general scheme of the application
    InvalidState = -3,
    /// Failed operation, mostly unknown reason!
    InvalidArgument = -2,
    /// System resource (file, image) not found
    Unspecified = -1,
    /// Image format error when opening an image of an unsupported format
    Duplicate = 1,
    /// Error filling table
    NotFound = 2,
}

impl std::error::Error for WvError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WvError::IoError(err) => Some(err),
            WvError::NullError(err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for WvError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            WvError::IoError(ref err) => err.fmt(f),
            WvError::NullError(ref err) => err.fmt(f),
            WvError::Internal(ref err) => write!(f, "An internal error occurred {:?}", err),
            WvError::EnvVarError(ref err) => write!(f, "An env var error occurred {:?}", err),
            WvError::Utf8Error(ref err) => {
                write!(f, "A UTF8 conversion error occurred {:?}", err)
            }
            WvError::ParseIntError(ref err) => {
                write!(f, "An int parsing error occurred {:?}", err)
            }
            WvError::Unknown(ref err) => write!(f, "An unknown error occurred {:?}", err),
        }
    }
}

impl From<io::Error> for WvError {
    fn from(err: io::Error) -> WvError {
        WvError::IoError(err)
    }
}

impl From<std::ffi::NulError> for WvError {
    fn from(err: std::ffi::NulError) -> WvError {
        WvError::NullError(err)
    }
}

impl From<std::env::VarError> for WvError {
    fn from(err: std::env::VarError) -> WvError {
        WvError::EnvVarError(err)
    }
}

impl From<std::string::FromUtf8Error> for WvError {
    fn from(err: std::string::FromUtf8Error) -> WvError {
        WvError::Utf8Error(err)
    }
}

impl From<std::num::ParseIntError> for WvError {
    fn from(err: std::num::ParseIntError) -> WvError {
        WvError::ParseIntError(err)
    }
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SizeHint {
    None = 0,
    Min = 1,
    Max = 2,
    Fixed = 3,
}

/// Webview wrapper
#[derive(Clone)]
pub struct Webview {
    inner: Arc<webview_t>,
}

unsafe impl Send for Webview {}
unsafe impl Sync for Webview {}

impl Drop for Webview {
    fn drop(&mut self) {
        if Arc::strong_count(&self.inner) == 0 {
            unsafe {
                webview_terminate(*self.inner);
                webview_destroy(*self.inner);
            }
        }
    }
}

impl Webview {
    /// Create a new instance of the webview
    pub fn create_no_win(debug: bool) -> Webview {
        Webview {
            inner: Arc::new(unsafe { webview_create(debug as raw::c_int, ptr::null_mut()) }),
        }
    }
    /// Navigate to a url
    pub fn navigate(&self, url: &str) -> Result<(), WvError> {
        let url = CString::new(url)?;
        unsafe {
            let ret = webview_navigate(*self.inner, url.as_ptr() as _);
            if ret != 0 {
                Err(WvError::Internal(std::mem::transmute(ret)))
            } else {
                Ok(())
            }
        }
    }

    /// Set the html content of the weview window
    pub fn set_html(&self, html: &str) -> Result<(), WvError> {
        // MS Edge chromium based also requires utf-8
        self.navigate(&(String::from("data:text/html;charset=utf-8,") + html))
    }

    /// Injects JavaScript code at the initialization of the new page
    pub fn init(&self, js: &str) -> Result<(), WvError> {
        let js = CString::new(js)?;
        unsafe {
            let ret = webview_init(*self.inner, js.as_ptr());
            if ret != 0 {
                Err(WvError::Internal(std::mem::transmute(ret)))
            } else {
                Ok(())
            }
        }
    }

    /// Evaluates arbitrary JavaScript code. Evaluation happens asynchronously
    pub fn eval(&self, js: &str) -> Result<(), WvError> {
        let js = CString::new(js)?;
        unsafe {
            let ret = webview_eval(*self.inner, js.as_ptr());
            if ret != 0 {
                Err(WvError::Internal(std::mem::transmute(ret)))
            } else {
                Ok(())
            }
        }
    }

    /// Posts a function to be executed on the main thread
    pub fn dispatch<F>(&mut self, f: F) -> Result<(), WvError>
    where
        F: FnOnce(Webview) + Send + 'static,
    {
        let closure = Box::into_raw(Box::new(f));
        extern "C" fn callback<F>(webview: webview_t, arg: *mut raw::c_void)
        where
            F: FnOnce(Webview) + Send + 'static,
        {
            let webview = Webview {
                inner: Arc::new(webview),
            };
            let closure: Box<F> = unsafe { Box::from_raw(arg as *mut F) };
            (*closure)(webview);
        }
        unsafe { let ret = webview_dispatch(*self.inner, Some(callback::<F>), closure as *mut _);
            if ret == 0 {
                Ok(())
            } else {
                Err(WvError::Internal(std::mem::transmute(ret)))
            }
        }
    }

    /// Binds a native C callback so that it will appear under the given name as a global JavaScript function
    pub fn bind<F>(&self, name: &str, f: F) -> Result<(), WvError>
    where
        F: FnMut(&str, &str),
    {
        let name = CString::new(name)?;
        let closure = Box::new(f);
        extern "C" fn callback<F: FnMut(&str, &str)>(
            seq: *const raw::c_char,
            req: *const raw::c_char,
            arg: *mut raw::c_void,
        ) {
            let seq = unsafe {
                CStr::from_ptr(seq)
                    .to_str()
                    .expect("No null bytes in parameter seq")
            };
            let req = unsafe {
                CStr::from_ptr(req)
                    .to_str()
                    .expect("No null bytes in parameter req")
            };
            let mut f: Box<F> = unsafe { Box::from_raw(arg as *mut F) };
            (*f)(seq, req);
            mem::forget(f);
        }
        unsafe {
            let ret = webview_bind(
                *self.inner,
                name.as_ptr(),
                Some(callback::<F>),
                Box::into_raw(closure) as *mut _,
            );
            if ret != 0 {
                Err(WvError::Internal(std::mem::transmute(ret)))
            } else {
                Ok(())
            }
        }
    }

    /// Unbinds a native C callback so that it will appear under the given name as a global JavaScript function
    pub fn unbind(&self, name: &str) -> Result<(), WvError> {
        let name = CString::new(name)?;
        let ret = unsafe { webview_unbind(*self.inner, name.as_ptr()) };
        if ret != 0 {
            Err(WvError::Internal(unsafe { std::mem::transmute(ret) }))
        } else {
            Ok(())
        }
    }

    /// Allows to return a value from the native binding.
    pub fn return_(&self, seq: &str, status: i32, result: &str) -> Result<(), WvError> {
        let seq = CString::new(seq)?;
        let result = CString::new(result)?;
        unsafe { 
            let ret = webview_return(*self.inner, seq.as_ptr(), status, result.as_ptr());
            if ret != 0 {
                Err(WvError::Internal(std::mem::transmute(ret)))
            } else {
                Ok(())
            }
        }
    }

    /// Set the size of the webview window
    pub fn set_size(&self, width: i32, height: i32, hints: SizeHint) -> Result<(), WvError> {
        unsafe { 
            let ret = webview_set_size(*self.inner, width, height, hints as u32);
            if ret != 0 {
                Err(WvError::Internal(std::mem::transmute(ret)))
            } else {
                Ok(())
            }
        }
    }

    /// Set the title
    pub fn set_title(&mut self, title: &str) -> Result<(), WvError> {
        let title = CString::new(title)?;
        unsafe { 
            let ret = webview_set_title(*self.inner, title.as_ptr());
            if ret != 0 {
                Err(WvError::Internal(std::mem::transmute(ret)))
            } else {
                Ok(())
            }
        }
    }

    /// Run the webview
    pub fn run(&mut self) -> Result<(), WvError> {
        unsafe {
            let ret = webview_run(*self.inner);
            if ret == 0 {
                Ok(())
            } else {
                Err(WvError::Internal(std::mem::transmute(ret)))
            }
        }
    }

    /// Get the webview's window
    pub fn get_window(&self) -> *mut raw::c_void {
        unsafe { webview_get_window(*self.inner) as *mut _ }
    }

    /// Create a Webview from an `Arc<webview_t>`
    pub fn from_raw(inner: Arc<webview_t>) -> Webview {
        Self { inner }
    }
}
