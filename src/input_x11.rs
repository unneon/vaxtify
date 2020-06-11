use std::ffi::{CStr, CString};
use std::mem::size_of;
use std::os::raw::{c_char, c_int, c_uint, c_ulong, c_void};
use std::ptr::{null, null_mut};
use x11::xlib::{AnyPropertyType, Atom, Display as RawDisplay, Success, Window, XGetAtomName, XWindowAttributes};

pub struct Display(*mut RawDisplay);

pub struct QueryTree {
	pub parent: Window,
	pub root: Window,
	pub children: Vec<Window>,
}

pub struct Property<T> {
	pub kind: Atom,
	pub kind_name: String,
	pub items: Vec<T>,
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

	pub fn get_window_pid(&self, window: Window) -> Option<u32> {
		let prop = self.get_window_property::<u32>(window, "_NET_WM_PID").unwrap()?;
		assert_eq!(prop.kind_name, "CARDINAL");
		assert_eq!(prop.items.len(), 1);
		Some(prop.items[0])
	}

	pub fn get_window_property<T: Clone>(&self, window: Window, name: &str) -> Result<Option<Property<T>>, ()> {
		let name = CString::new(name).unwrap();
		let atom = unsafe { x11::xlib::XInternAtom(self.0, name.as_ptr(), x11::xlib::True) };
		if atom == 0 {
			return Err(());
		}
		let mut kind: Atom = 0;
		let mut format: c_int = 0;
		let mut items_len: c_ulong = 0;
		let mut bytes: c_ulong = 0;
		let mut data: *mut u8 = null_mut();
		let status = unsafe {
			x11::xlib::XGetWindowProperty(
				self.0,
				window,
				atom,
				0,
				!0,
				x11::xlib::False,
				AnyPropertyType as c_ulong,
				&mut kind,
				&mut format,
				&mut items_len,
				&mut bytes,
				&mut data,
			)
		};
		if status != Success as i32 {
			return Err(());
		}
		if kind == 0 {
			return Ok(None);
		}
		assert_eq!(format as usize, 8 * size_of::<T>());
		assert_eq!(bytes, 0);
		let property = Property {
			kind,
			kind_name: self.get_atom_name(kind),
			items: unsafe { xarray_into_vec::<T>(data as *mut T, items_len as c_uint) },
		};
		Ok(Some(property))
	}

	fn get_atom_name(&self, atom: Atom) -> String {
		let raw = unsafe { XGetAtomName(self.0, atom) };
		// FIXME: Check for error.
		unsafe { xstr_into_string(raw) }
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
