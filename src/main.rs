use std::os::raw::{c_char, c_void};

extern crate glib;
extern crate glib_sys;

use glib_sys::GError;
use std::ffi::{CString, OsString};
use std::{fs, path, ptr};

struct PopplerDocumentRef(*mut ffi::PopplerDocument);

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


impl PopplerDocumentRef {
    fn new_from_file<P: AsRef<path::Path>>(p: P, password: &str) -> Result<(), glib::error::Error> {
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

        Ok(())
    }
}


#[derive(Debug)]
pub struct PoppperPageRef {
    ptr: *mut c_void,
}

mod ffi {
    use std::os::raw::c_char;
    use glib_sys;

    // FIXME: is this the correct way to get opaque types?
    // NOTE: https://github.com/rust-lang/rust/issues/27303
    // NOTE: ask F/O about this
    pub enum PopplerDocument {}

    #[link(name = "poppler-glib")]
    extern "C" {
        pub fn poppler_document_new_from_file(
            uri: *const c_char,
            password: *const c_char,
            error: *mut *mut glib_sys::GError,
        ) -> *mut PopplerDocument;
    }
}


fn main() {
    let filename = "test.pdf";

    // int main(int argc, char *argv[])
    // {


    //     double width, height;
    //     GError *error;
    //     const char *filename;
    //     gchar *absolute, *uri;
    //     int num_pages, i;
    //     cairo_surface_t *surface;
    //     cairo_t *cr;
    //     cairo_status_t status;

    match PopplerDocumentRef::new_from_file(filename, "") {
        Ok(_) => {}
        Err(e) => println!("ERROR: {}", e),
    };

    //     document = poppler_document_new_from_file (uri, NULL, &error);
    //     if (document == NULL) {
    //         printf("poppler fail: %s\n", error->message);
    //         return 1;
    //     }

    //     num_pages = poppler_document_get_n_pages (document);

    //      Page size does not matter here as the size is changed before
    //      * each page
    //     surface = cairo_ps_surface_create ("output.ps", 595, 842);
    //     cr = cairo_create (surface);
    //     for (i = 0; i < num_pages; i++) {
    //         page = poppler_document_get_page (document, i);
    //         if (page == NULL) {
    //             printf("poppler fail: page not found\n");
    //             return 1;
    //         }
    //         poppler_page_get_size (page, &width, &height);
    //         cairo_ps_surface_set_size (surface, width, height);
    //         cairo_save (cr);
    //         poppler_page_render_for_printing (page, cr);
    //         cairo_restore (cr);
    //         cairo_surface_show_page (surface);
    //         g_object_unref (page);
    //     }
    //     status = cairo_status(cr);
    //     if (status)
    //         printf("%s\n", cairo_status_to_string (status));
    //     cairo_destroy (cr);
    //     cairo_surface_finish (surface);
    //     status = cairo_surface_status(surface);
    //     if (status)
    //         printf("%s\n", cairo_status_to_string (status));
    //     cairo_surface_destroy (surface);

    //     g_object_unref (document);

    //     return 0;
    // }
}
