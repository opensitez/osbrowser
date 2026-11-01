//! Interactive Canvas demonstration showing both vector primitives and raw pixel blitting
//! running directly on the system browser via osbrowser's WHATWG Canvas bridge.
//!
//! Run with:
//!     cargo run --example canvas

use std::f32::consts::PI;
use osbrowser::browser::{Browser, BrowserDocument, BrowserGuest, BrowserOptions};
use osbrowser::canvas::{Color, Image};

struct CanvasPage;

impl BrowserGuest for CanvasPage {
    fn dispatch_pending(&mut self) -> bool {
        false
    }
}

fn main() {
    let document = BrowserDocument::new(osbrowser::load_html(
        "<!DOCTYPE html>\
        <html>\
        <head>\
            <title>osbrowser - Canvas & Blit Demo</title>\
            <style>\
                body {\
                    margin: 0;\
                    padding: 20px;\
                    background-color: #0b0f19;\
                    display: flex;\
                    flex-direction: column;\
                    align-items: center;\
                    justify-content: center;\
                    min-height: 100vh;\
                    font-family: system-ui, -apple-system, sans-serif;\
                    color: #f1f5f9;\
                }\
                h1 {\
                    margin-top: 0;\
                    margin-bottom: 12px;\
                    font-size: 24px;\
                    color: #38bdf8;\
                }\
                canvas {\
                    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.6);\
                    border-radius: 8px;\
                    background-color: #0f172a;\
                }\
            </style>\
        </head>\
        <body>\
            <h1>osbrowser: Native Canvas Primitives & Pixel Blitting</h1>\
            <canvas id=\"main_canvas\" width=\"800\" height=\"600\"></canvas>\
        </body>\
        </html>",
        800.0,
    ));

    {
        let mut doc = document.write();
        let canvas_id = doc
            .get_element_by_id("main_canvas")
            .expect("canvas element '#main_canvas' must exist in HTML");

        // Draw both vector primitives and raw RGBA pixel blits
        doc.with_canvas_2d(canvas_id, |ctx| {
            // 1. Dark canvas background
            ctx.set_fill_color(Color { r: 15, g: 23, b: 42, a: 255 });
            ctx.fill_rect(0.0, 0.0, 800.0, 600.0);

            // 2. Header banner
            ctx.set_fill_color(Color { r: 30, g: 41, b: 59, a: 255 });
            ctx.fill_rect(20.0, 20.0, 760.0, 52.0);

            ctx.set_font_css("bold 20px system-ui, sans-serif");
            ctx.set_fill_color(Color { r: 241, g: 245, b: 249, a: 255 });
            ctx.fill_text("Real System Browser Canvas: Vector Primitives + Pixel Blitting", 40.0, 53.0);

            // ── Left Card: Vector Primitives ──────────────────────────────
            ctx.set_stroke_color(Color { r: 59, g: 130, b: 246, a: 255 });
            ctx.set_line_width(2.0);
            ctx.stroke_rect(30.0, 90.0, 350.0, 480.0);

            ctx.set_font_css("bold 18px system-ui, sans-serif");
            ctx.set_fill_color(Color { r: 96, g: 165, b: 250, a: 255 });
            ctx.fill_text("1. Vector Primitives (GPU)", 50.0, 130.0);

            // Draw circles with path
            ctx.set_fill_color(Color { r: 239, g: 68, b: 68, a: 220 });
            ctx.begin_path();
            ctx.arc(150.0, 220.0, 55.0, 0.0, PI * 2.0, false);
            ctx.fill();

            ctx.set_fill_color(Color { r: 59, g: 130, b: 246, a: 180 });
            ctx.begin_path();
            ctx.arc(200.0, 250.0, 55.0, 0.0, PI * 2.0, false);
            ctx.fill();

            // Draw polygon / triangle
            ctx.set_fill_color(Color { r: 16, g: 185, b: 129, a: 255 });
            ctx.begin_path();
            ctx.move_to(205.0, 330.0);
            ctx.line_to(120.0, 450.0);
            ctx.line_to(290.0, 450.0);
            ctx.close_path();
            ctx.fill();

            ctx.set_font_css("14px system-ui, sans-serif");
            ctx.set_fill_color(Color { r: 203, g: 213, b: 225, a: 255 });
            ctx.fill_text("fillRect, arc, beginPath, closePath, fill, stroke", 50.0, 510.0);

            // ── Right Card: Raw Pixel Blitting (putImageData) ────────────
            ctx.set_stroke_color(Color { r: 168, g: 85, b: 247, a: 255 });
            ctx.set_line_width(2.0);
            ctx.stroke_rect(420.0, 90.0, 350.0, 480.0);

            ctx.set_font_css("bold 18px system-ui, sans-serif");
            ctx.set_fill_color(Color { r: 192, g: 132, b: 252, a: 255 });
            ctx.fill_text("2. Raw Pixel Blitting (putImageData)", 440.0, 130.0);

            // Generate procedural 256x256 RGBA pixel buffer
            let w = 256u32;
            let h = 256u32;
            let mut pixels = Vec::with_capacity((w * h * 4) as usize);
            for y in 0..h {
                for x in 0..w {
                    let r = ((x ^ y) & 0xff) as u8;
                    let g = (((x as u64 * y as u64) / 128) & 0xff) as u8;
                    let b = (255u32.saturating_sub(x.abs_diff(y))) as u8;
                    let a = 255u8;
                    pixels.extend_from_slice(&[r, g, b, a]);
                }
            }

            // Blit raw bytes directly to browser canvas via binary HTTP channel
            let img = Image::from_rgba(w, h, pixels);
            ctx.put_image_data(&img, 467.0, 170.0);

            ctx.set_font_css("14px system-ui, sans-serif");
            ctx.set_fill_color(Color { r: 203, g: 213, b: 225, a: 255 });
            ctx.fill_text("256x256 raw RGBA byte buffer", 480.0, 470.0);
            ctx.set_font_css("12px monospace");
            ctx.set_fill_color(Color { r: 148, g: 163, b: 184, a: 255 });
            ctx.fill_text("Binary application/octet-stream -> ImageData", 445.0, 510.0);
        });
    }

    let mut browser = Browser::open(BrowserOptions {
        title: "osbrowser - Canvas Primitives & Blitting Demo".into(),
        ..Default::default()
    })
    .expect("failed to open system browser");

    browser.attach_document(document);
    browser.set_guest(CanvasPage);
    println!("Canvas sample running in system browser tab. Close window or Ctrl+C to exit.");
    browser.run().expect("browser loop failed");
}
