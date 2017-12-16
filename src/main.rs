use std::os::raw::{c_double, c_int, c_void};

extern crate cairo;
extern crate cairo_sys;
extern crate glib;
extern crate glib_sys;

use cairo::prelude::SurfaceExt;

use glib_sys::GError;
use std::ffi::{CString, OsString};
use std::{fs, path, ptr};

#[derive(Debug)]
struct PopplerDocument(*mut ffi::PopplerDocument);

#[derive(Debug)]
struct PopplerPage(*mut ffi::PopplerPage);

fn call_with_gerror<T, F>(f: F) -> Result<*mut T, glib::error::Error>
where
    F: FnOnce(*mut *mut GError) -> *mut T,
{
    // initialize error to a null-pointer
    let mut err = ptr::null_mut();

    // call the c-library function
    let return_value = f(&mut err as *mut *mut GError);

    if return_value.is_null() {
        Err(glib::error::Error::wrap(err))
    } else {
        Ok(return_value)
    }
}


fn path_to_glib_url<P: AsRef<path::Path>>(p: P) -> Result<CString, glib::error::Error> {
    // canonicalize path, try to wrap failures into a glib error
    let canonical = fs::canonicalize(p).map_err(|_| {
        glib::error::Error::new(
            glib::FileError::Noent,
            "Could not turn path into canonical path. Maybe it does not exist?",
        )
    })?;

    // construct path string
    let mut osstr_path: OsString = "file:///".into();
    osstr_path.push(canonical);

    // we need to round-trip to string, as not all os strings are 8 bytes
    let pdf_string = osstr_path.into_string().map_err(|_| {
        glib::error::Error::new(
            glib::FileError::Inval,
            "Path invalid (contains non-utf8 characters)",
        )
    })?;

    CString::new(pdf_string).map_err(|_| {
        glib::error::Error::new(
            glib::FileError::Inval,
            "Path invalid (contains NUL characters)",
        )
    })
}


impl PopplerDocument {
    pub fn new_from_file<P: AsRef<path::Path>>(
        p: P,
        password: &str,
    ) -> Result<PopplerDocument, glib::error::Error> {
        let pw = CString::new(password).map_err(|_| {
            glib::error::Error::new(
                glib::FileError::Inval,
                "Password invalid (possibly contains NUL characters)",
            )
        })?;

        let path_cstring = path_to_glib_url(p)?;
        let doc = call_with_gerror(|err_ptr| unsafe {
            ffi::poppler_document_new_from_file(path_cstring.as_ptr(), pw.as_ptr(), err_ptr)
        })?;

        Ok(PopplerDocument(doc))
    }

    pub fn get_n_pages(&self) -> usize {
        // FIXME: what's the correct type here? can we assume a document
        //        has a positive number of pages?
        (unsafe { ffi::poppler_document_get_n_pages(self.0) }) as usize
    }

    pub fn get_page(&self, index: usize) -> Option<PopplerPage> {
        match unsafe { ffi::poppler_document_get_page(self.0, index as c_int) } {
            ptr if ptr.is_null() => None,
            ptr => Some(PopplerPage(ptr)),
        }
    }
}


impl PopplerPage {
    pub fn get_size(&self) -> (f64, f64) {
        let mut width: f64 = 0.0;
        let mut height: f64 = 0.0;

        unsafe {
            ffi::poppler_page_get_size(
                self.0,
                &mut width as *mut f64 as *mut c_double,
                &mut height as *mut f64 as *mut c_double,
            )
        }

        (width, height)
    }

    pub fn render_for_printing(&self, ctx: &mut cairo::Context) {
        unsafe { ffi::poppler_page_render_for_printing(self.0, ctx.to_raw_none()) }
    }
}


#[derive(Debug)]
pub struct PoppperPageRef {
    ptr: *mut c_void,
}

mod ffi {
    use std::os::raw::{c_char, c_double, c_int};
    use cairo_sys;
    use glib_sys;

    // FIXME: is this the correct way to get opaque types?
    // FIXME: alternative: https://docs.rs/cairo-sys-rs/0.5.0/src/cairo_sys/lib.rs.html#64
    // NOTE: https://github.com/rust-lang/rust/issues/27303
    // NOTE: ask F/O about this
    pub enum PopplerDocument {}
    pub enum PopplerPage {}

    // FIXME: *const instead of mut pointers?

    #[link(name = "poppler-glib")]
    extern "C" {
        pub fn poppler_document_new_from_file(
            uri: *const c_char,
            password: *const c_char,
            error: *mut *mut glib_sys::GError,
        ) -> *mut PopplerDocument;
        pub fn poppler_document_get_n_pages(document: *mut PopplerDocument) -> c_int;
        pub fn poppler_document_get_page(
            document: *mut PopplerDocument,
            index: c_int,
        ) -> *mut PopplerPage;

        pub fn poppler_page_get_size(
            page: *mut PopplerPage,
            width: *mut c_double,
            height: *mut c_double,
        );
        pub fn poppler_page_render_for_printing(
            page: *mut PopplerPage,
            cairo: *mut cairo_sys::cairo_t,
        );

        // FIXME: needs to be in upstream version of cairo-rs
        pub fn cairo_pdf_surface_set_size(
            surface: *mut cairo_sys::cairo_surface_t,
            width_in_points: c_double,
            height_in_points: c_double,
        );
    }
}


// FIXME: needs to be in upstream version of cairo-rs
pub trait CairoSetSize {
    fn set_size(&mut self, width_in_points: f64, height_in_points: f64);
}

impl CairoSetSize for cairo::PDFSurface {
    // FIXME: does this need mut?
    fn set_size(&mut self, width_in_points: f64, height_in_points: f64) {
        unsafe {
            ffi::cairo_pdf_surface_set_size(
                self.to_raw_none(),
                width_in_points as c_double,
                height_in_points as c_double,
            )
        }
    }
}


fn run() -> Result<(), glib::error::Error> {
    let filename = "test.pdf";
    let doc = PopplerDocument::new_from_file(filename, "")?;
    let num_pages = doc.get_n_pages();

    println!("Document has {} page(s)", num_pages);

    let mut surface = cairo::PDFSurface::create("output.pdf", 420.0, 595.0);
    let mut ctx = cairo::Context::new(&mut surface);

    // FIXME: move iterator to poppler
    for page_num in 0..num_pages {
        let page = doc.get_page(page_num).unwrap();
        let (w, h) = page.get_size();
        println!("page {} has size {}, {}", page_num, w, h);
        // surface.set_size(w as i32, h as i32);  // ??

        ctx.save();
        page.render_for_printing(&mut ctx);
        ctx.restore();
        ctx.show_page();
    }
    //         g_object_unref (page);

    surface.finish();

    Ok(())
}


fn main() {
    match run() {
        Ok(()) => (),
        Err(e) => {
            println!("ERROR: {}", e);
        }
    };
}
