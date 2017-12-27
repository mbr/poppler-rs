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
    pub fn poppler_page_render_for_printing(page: *mut PopplerPage, cairo: *mut cairo_sys::cairo_t);

    // FIXME: needs to be in upstream version of cairo-rs
    pub fn cairo_pdf_surface_set_size(
        surface: *mut cairo_sys::cairo_surface_t,
        width_in_points: c_double,
        height_in_points: c_double,
    );
}
