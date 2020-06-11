use std::ffi::CStr;
use std::os::raw::{c_char, c_uint, c_void};
use std::ptr::{null, null_mut};

use x11::xlib::{Display as RawDisplay, Window, XWindowAttributes};

pub struct Display(*mut RawDisplay);

pub struct QueryTree {
	pub parent: Window,
	pub root: Window,
	pub children: Vec<Window>,
}

impl Display {
	pub fn open() -> Result<Display, ()> {
		let raw = unsafe { x11::xlib::XOpenDisplay(null()) };
		if !raw.is_null() {
			Ok(Display(raw))
		} else {
			Err(())
		}
	}

	pub fn query_tree(&self) -> Result<QueryTree, ()> {
		// Can't find docs whether this can fail.
		let default_root = unsafe { x11::xlib::XDefaultRootWindow(self.0) };
		let mut parent = 0;
		let mut root = 0;
		let mut children = null_mut();
		let mut children_len = 0;
		let ret = unsafe {
			x11::xlib::XQueryTree(self.0, default_root, &mut root, &mut parent, &mut children, &mut children_len)
		};
		if ret == 0 {
			return Err(());
		}
		let children = unsafe { xarray_into_vec(children, children_len) };
		Ok(QueryTree { parent, root, children })
	}

	pub fn get_window_attributes(&self, window: Window) -> Result<XWindowAttributes, ()> {
		let mut attributes: XWindowAttributes = unsafe { std::mem::zeroed() };
		let ret = unsafe { x11::xlib::XGetWindowAttributes(self.0, window, &mut attributes) };
		if ret == 0 {
			return Err(());
		}
		Ok(attributes)
	}

	pub fn get_window_name(&self, window: Window) -> Result<String, ()> {
		let mut name: *mut c_char = null_mut();
		let ret = unsafe { x11::xlib::XFetchName(self.0, window, &mut name) };
		if ret == 0 {
			return Err(());
		}
		let name = unsafe { xstr_into_string(name) };
		Ok(name)
	}
}

impl Drop for Display {
	fn drop(&mut self) {
		// XCloseDisplay is hardcoded to return 0.
		unsafe { x11::xlib::XCloseDisplay(self.0) };
	}
}

unsafe fn xarray_into_vec<T: Clone>(data: *mut T, len: c_uint) -> Vec<T> {
	assert!(!data.is_null());
	let vec = std::slice::from_raw_parts(data, len as usize).to_owned();
	x11::xlib::XFree(data as *mut c_void);
	vec
}

unsafe fn xstr_into_string(data: *mut c_char) -> String {
	assert!(!data.is_null());
	let bytes = CStr::from_ptr(data).to_bytes();
	let str = std::str::from_utf8(bytes).unwrap().to_owned();
	x11::xlib::XFree(data as *mut c_void);
	str
}
