extern crate cairo;
extern crate cairo_sys;
extern crate glib;
extern crate glib_sys;

mod ffi;
mod util;

use cairo::prelude::SurfaceExt;

use std::ffi::CString;
use std::os::raw::{c_double, c_int, c_void};
use std::path;

#[derive(Debug)]
struct PopplerDocument(*mut ffi::PopplerDocument);

#[derive(Debug)]
struct PopplerPage(*mut ffi::PopplerPage);


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

        let path_cstring = util::path_to_glib_url(p)?;
        let doc = util::call_with_gerror(|err_ptr| unsafe {
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
