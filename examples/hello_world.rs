//! A minimal Hello World page shown in the system browser.
//!
//! Run with:
//!     cargo run --example hello_world

use osbrowser::browser::{Browser, BrowserDocument, BrowserGuest, BrowserOptions};

/// Guest handler for the browser event loop.
struct Page;

impl BrowserGuest for Page {
    fn dispatch_pending(&mut self) -> bool {
        false
    }
}

fn main() {
    let document = BrowserDocument::new(osbrowser::load_html(
        "<!DOCTYPE html>\
        <html>\
        <head>\
            <title>Hello World</title>\
            <style>\
                body {\
                    font-family: system-ui, -apple-system, sans-serif;\
                    display: flex;\
                    flex-direction: column;\
                    align-items: center;\
                    justify-content: center;\
                    height: 100vh;\
                    margin: 0;\
                    background-color: #f7f9fa;\
                    color: #1a1a1a;\
                }\
                h1 {\
                    font-size: 2.5rem;\
                    margin-bottom: 0.5rem;\
                }\
                p {\
                    font-size: 1.1rem;\
                    color: #666;\
                }\
            </style>\
        </head>\
        <body>\
            <h1>Hello, World!</h1>\
            <p>Rendered in your system browser via osbrowser.</p>\
        </body>\
        </html>",
        800.0,
    ));

    let mut browser = Browser::open(BrowserOptions {
        title: "Hello World - osbrowser".into(),
        ..Default::default()
    })
    .expect("failed to open the browser");

    browser.attach_document(document);
    browser.set_guest(Page);
    browser.run().expect("failed to run the browser");
}
